//! The `sysinfo`-backed sampler that gathers a `MetricsState` every update interval and
//! sends it on the hub's metrics channel.
use crate::features::metrics::domain::{
    CpuSample, CreateMetricsCommand, DeviceMetrics, DiskMetric, DiskSpace, MemoryBytes,
    MemoryMetrics, MemoryUsage, MetricsConfig, MetricsState, SystemMetrics, Temperature,
};
use crate::shared::events::signals::SignalHub;
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
        let Self {
            sys,
            networks,
            disks,
            components,
        } = self;
        sys.refresh_cpu_usage();
        sys.refresh_memory();
        if config.network().is_some()
            && config.network() != Some(&crate::features::metrics::domain::NetworkMode::Disabled)
        {
            networks.refresh(true);
        }
        if config.disk().is_some()
            && config.disk() != Some(&crate::features::metrics::domain::DiskMode::Disabled)
        {
            disks.refresh(true);
        }
        if config.temperature().is_some()
            && config.temperature()
                != Some(&crate::features::metrics::domain::TemperatureMode::Disabled)
        {
            components.refresh(true);
        }

        // CPU
        let nproc = f32::from(u16::try_from(sys.cpus().len()).unwrap_or(1));
        let global_cpu = sys.global_cpu_usage();

        let per_core_raw: Vec<f32> = sys.cpus().iter().map(sysinfo::Cpu::cpu_usage).collect();
        let cpu = CpuSample::new(global_cpu, nproc, per_core_raw).normalize(config.cpu());

        // Network
        let mut network_tx: u64 = 0;
        let mut network_rx: u64 = 0;
        if config.network().is_some()
            && config.network() != Some(&crate::features::metrics::domain::NetworkMode::Disabled)
        {
            for data in networks.values() {
                network_tx = network_tx.saturating_add(data.transmitted());
                network_rx = network_rx.saturating_add(data.received());
            }
        }

        // Disks
        let mut disk_metrics = Vec::new();
        if config.disk().is_some()
            && config.disk() != Some(&crate::features::metrics::domain::DiskMode::Disabled)
        {
            for disk in disks.iter() {
                disk_metrics.push(DiskMetric::new(
                    crate::features::metrics::domain::DiskName::new(disk.name().to_string_lossy()),
                    crate::features::metrics::domain::MountPoint::new(
                        disk.mount_point().to_string_lossy(),
                    ),
                    DiskSpace::new(
                        MemoryBytes::new(disk.total_space()),
                        MemoryBytes::new(disk.available_space()),
                        MemoryBytes::new(disk.total_space().saturating_sub(disk.available_space())),
                    ),
                ));
            }
        }

        // Temperature
        let temperature = config.temperature().and_then(|mode| {
            Temperature::from_readings(components.iter().map(sysinfo::Component::temperature), mode)
        });

        MetricsState::new(CreateMetricsCommand::new(
            SystemMetrics::new(
                cpu,
                MemoryMetrics::new(
                    MemoryUsage::new(
                        MemoryBytes::new(sys.used_memory()),
                        MemoryBytes::new(sys.total_memory()),
                    ),
                    MemoryUsage::new(
                        MemoryBytes::new(sys.used_swap()),
                        MemoryBytes::new(sys.total_swap()),
                    ),
                ),
            ),
            DeviceMetrics::new(
                disk_metrics,
                crate::features::metrics::domain::NetworkMetrics::new(
                    crate::features::metrics::domain::NetworkSpeed::new(network_tx),
                    crate::features::metrics::domain::NetworkSpeed::new(network_rx),
                ),
                temperature,
            ),
            config.clone(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
