//! The `sysinfo`-backed sampler that gathers a `MetricsState` every update interval and
//! sends it on the hub's metrics channel.
mod readings;

use crate::features::metrics::domain::{
    CpuMetrics, CreateMetricsCommand, DeviceMetrics, DiskMetric, DiskName, DiskSpace, MemoryBytes,
    MemoryMetrics, MemoryUsage, MetricsConfig, MetricsState, MountPoint, NetworkMetrics,
    SystemMetrics, Temperature,
};
use crate::shared::events::signals::SignalHub;
use readings::{
    average_temperature, cpu_metrics, disk_enabled, network_enabled, network_metrics,
    temperature_enabled,
};
use std::sync::Arc;
use sysinfo::{Components, Disks, Networks, System};

pub(crate) struct SysinfoAdapter {
    config: MetricsConfig,
    hub: Arc<SignalHub>,
}

impl SysinfoAdapter {
    #[must_use]
    pub(crate) const fn new(config: MetricsConfig, hub: Arc<SignalHub>) -> Self {
        Self { config, hub }
    }

    pub(crate) fn start(&self) {
        let config = self.config.clone();
        let hub = self.hub.clone();

        tokio::spawn(async move {
            let mut sources = SysinfoSources::new();

            loop {
                let state = sources.gather(&config);

                if hub.metrics_tx().send(state).is_err() {
                    // Receiver dropped
                    break;
                }

                tokio::time::sleep(std::time::Duration::from_millis(
                    config.update_interval_ms().value(),
                ))
                .await;
            }
        });
    }
}

/// The sysinfo readers a sampling pass refreshes: the host (`System`, for CPU and memory) and
/// the three device lists (disks, network interfaces and sensor components).
struct SysinfoSources {
    sys: System,
    networks: Networks,
    disks: Disks,
    components: Components,
}

impl SysinfoSources {
    /// Readers for this machine, with the device lists already enumerated.
    #[must_use]
    fn new() -> Self {
        Self {
            sys: System::new(),
            networks: Networks::new_with_refreshed_list(),
            disks: Disks::new_with_refreshed_list(),
            components: Components::new_with_refreshed_list(),
        }
    }

    /// Refreshes what `config` enables and reads it into one snapshot.
    #[must_use]
    fn gather(&mut self, config: &MetricsConfig) -> MetricsState {
        self.refresh(config);
        let cpu = self.cpu(config);
        let network = self.network(config);
        let disks = self.disk_metrics(config);
        let temperature = self.temperature(config);
        MetricsState::new(CreateMetricsCommand::new(
            SystemMetrics::new(cpu, self.memory()),
            DeviceMetrics::new(disks, network, temperature),
            config.clone(),
        ))
    }

    /// Refreshes the CPU and memory readings, then each device list `config` enables.
    fn refresh(&mut self, config: &MetricsConfig) {
        self.sys.refresh_cpu_usage();
        self.sys.refresh_memory();
        if network_enabled(config) {
            self.networks.refresh(true);
        }
        if disk_enabled(config) {
            self.disks.refresh(true);
        }
        if temperature_enabled(config) {
            self.components.refresh(true);
        }
    }

    /// The overall and per-core CPU usage, scaled as `config` asks.
    fn cpu(&self, config: &MetricsConfig) -> CpuMetrics {
        let per_core = self
            .sys
            .cpus()
            .iter()
            .map(sysinfo::Cpu::cpu_usage)
            .collect();
        cpu_metrics(self.sys.global_cpu_usage(), per_core, config.cpu())
    }

    /// The bytes every interface sent and received, or zero both ways when network is off.
    fn network(&self, config: &MetricsConfig) -> NetworkMetrics {
        let pairs = self
            .networks
            .values()
            .map(|data| (data.transmitted(), data.received()));
        network_metrics(config, pairs)
    }

    /// One reading per disk, or none when disk is off.
    fn disk_metrics(&self, config: &MetricsConfig) -> Vec<DiskMetric> {
        if !disk_enabled(config) {
            return Vec::new();
        }
        self.disks.iter().map(disk_metric).collect()
    }

    /// The average sensor temperature in the configured unit, absent when temperature is
    /// not configured or nothing could be read.
    fn temperature(&self, config: &MetricsConfig) -> Option<Temperature> {
        let readings = self.components.iter().map(sysinfo::Component::temperature);
        average_temperature(config, readings)
    }

    /// The used and total RAM and swap.
    fn memory(&self) -> MemoryMetrics {
        let sys = &self.sys;
        MemoryMetrics::new(
            MemoryUsage::new(
                MemoryBytes::new(sys.used_memory()),
                MemoryBytes::new(sys.total_memory()),
            ),
            MemoryUsage::new(
                MemoryBytes::new(sys.used_swap()),
                MemoryBytes::new(sys.total_swap()),
            ),
        )
    }
}

/// The name, mount point and space of `disk`, its used bytes being total minus available.
fn disk_metric(disk: &sysinfo::Disk) -> DiskMetric {
    DiskMetric::new(
        DiskName::new(disk.name().to_string_lossy()),
        MountPoint::new(disk.mount_point().to_string_lossy()),
        DiskSpace::new(
            MemoryBytes::new(disk.total_space()),
            MemoryBytes::new(disk.available_space()),
            MemoryBytes::new(disk.total_space().saturating_sub(disk.available_space())),
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::metrics::domain::TemperatureMode;
    use crate::shared::config::domain::Config;

    #[test]
    fn test_sysinfo_adapter_gather_metrics_all_enabled() {
        let config: MetricsConfig = serde_json::from_value(serde_json::json!({
            "cpu": "percentage_0_100",
            "network": "tx_rx",
            "disk": "percentual",
            "temperature": "celsius",
            "update_interval_ms": 100
        }))
        .unwrap();

        let state = SysinfoSources::new().gather(&config);

        assert!(state.memory_total().value() > 0);
    }

    #[test]
    fn test_sysinfo_adapter_gather_metrics_all_disabled() {
        let config: MetricsConfig = serde_json::from_value(serde_json::json!({
            "cpu": "disabled",
            "network": "disabled",
            "disk": "disabled",
            "temperature": "disabled",
            "update_interval_ms": 100
        }))
        .unwrap();

        let state = SysinfoSources::new().gather(&config);

        assert_eq!(state.network_tx().value(), 0);
        assert_eq!(state.network_rx().value(), 0);
        assert_eq!(state.disks(), []);
        assert_eq!(state.temperature(), None);
    }

    /// A config read from `json`.
    fn config_from(json: serde_json::Value) -> MetricsConfig {
        serde_json::from_value(json).unwrap()
    }

    /// The `(total, available, used)` bytes of `disk`, as scripts see them.
    fn space_of(disk: &crate::features::metrics::domain::DiskMetric) -> (u64, u64, u64) {
        let json = serde_json::to_value(disk).unwrap();
        let bytes = |key: &str| json[key].as_u64().unwrap();
        (
            bytes("total_bytes"),
            bytes("available_bytes"),
            bytes("used_bytes"),
        )
    }

    #[test]
    fn test_gather_without_device_settings_reads_no_device() {
        let config = MetricsConfig::default();
        let state = SysinfoSources::new().gather(&config);

        let readings = (state.network_tx.value(), state.network_rx.value());
        assert_eq!(readings, (0, 0));
        assert_eq!((state.disks.len(), state.temperature), (0, None));
        assert_eq!(state.config, config);
    }

    #[test]
    fn test_gather_reads_memory_and_swap_of_the_host() {
        let mut host = System::new();
        host.refresh_memory();
        let state = SysinfoSources::new().gather(&MetricsConfig::default());

        let totals = (state.memory_total.value(), state.swap_total.value());
        assert_eq!(totals, (host.total_memory(), host.total_swap()));
        assert!(state.memory_used.value() > 0);
        assert!(state.memory_used.value() <= state.memory_total.value());
        assert!(state.swap_used.value() <= state.swap_total.value());
    }

    #[test]
    fn test_gather_scales_one_reading_per_core_with_the_configured_mode() {
        let config = config_from(serde_json::json!({ "cpu": "disabled" }));
        let mut sources = SysinfoSources::new();
        let state = sources.gather(&config);
        let cores = sources.sys.cpus().len();

        let expected = vec![crate::features::metrics::domain::CpuUsage::new(0.0); cores];
        assert!(cores > 0);
        assert_eq!(state.per_core, expected);
        assert_eq!(state.cpu_usage.value().to_bits(), 0.0_f32.to_bits());
    }

    /// Says on stderr that a host test checked nothing real when the host has no `what`
    /// (`found` is 0); the pure steps in `readings` are what cover the logic on such a host.
    fn say_if_vacuous(found: usize, what: &str) {
        if found == 0 {
            eprintln!("VACUOUS: this host has no {what}, so this test checked only its absence");
        }
    }

    #[test]
    fn test_gather_reads_each_disk_when_disk_is_enabled() {
        let state =
            SysinfoSources::new().gather(&config_from(serde_json::json!({ "disk": "absolute" })));
        let fresh = Disks::new_with_refreshed_list();
        say_if_vacuous(fresh.list().len(), "disk");

        assert_eq!(state.disks.len(), fresh.list().len());
        for (metric, disk) in state.disks.iter().zip(fresh.list()) {
            let (total, available, used) = space_of(metric);
            assert_eq!(metric.name().as_str(), disk.name().to_string_lossy());
            assert_eq!(
                metric.mount_point().as_str(),
                disk.mount_point().to_string_lossy()
            );
            assert_eq!(
                (total, used),
                (disk.total_space(), total.saturating_sub(available))
            );
            let fresh_used = disk.total_space().saturating_sub(disk.available_space());
            assert!(available.abs_diff(disk.available_space()) <= available.abs_diff(fresh_used));
        }
        assert_eq!((state.network_tx.value(), state.temperature), (0, None));
    }

    #[test]
    fn test_gather_reads_the_sensors_when_temperature_is_enabled() {
        let config = config_from(serde_json::json!({ "temperature": "celsius" }));
        let state = SysinfoSources::new().gather(&config);
        let fresh = Components::new_with_refreshed_list();
        let readings = fresh.iter().map(sysinfo::Component::temperature);
        let expected = Temperature::from_readings(readings, &TemperatureMode::Celsius);
        say_if_vacuous(usize::from(expected.is_some()), "sensor reading");

        assert_eq!(state.temperature.is_some(), expected.is_some());
        assert_eq!(state.disks.len(), 0);
    }

    #[test]
    fn test_sysinfo_adapter_start() {
        crate::test_utils::block_on(async {
            let config: MetricsConfig = serde_json::from_value(serde_json::json!({
                "update_interval_ms": 10
            }))
            .unwrap();
            let hub = Arc::new(SignalHub::new(Config::default()));
            let mut rx = hub.metrics_rx().clone();

            let adapter = SysinfoAdapter::new(config, hub);
            adapter.start();

            // Wait for first update
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            let state = rx.borrow_and_update().clone();
            assert!(state.memory_total().value() > 0);
        });
    }
}
