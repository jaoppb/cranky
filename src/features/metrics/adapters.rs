use crate::features::metrics::domain::{DiskMetric, MetricsConfig, MetricsState};
use crate::shared::events::signals::SignalHub;
use std::sync::Arc;
use sysinfo::{Components, Disks, Networks, System};

pub struct SysinfoAdapter {
    config: MetricsConfig,
    hub: Arc<SignalHub>,
}

impl SysinfoAdapter {
    #[must_use]
    pub const fn new(config: MetricsConfig, hub: Arc<SignalHub>) -> Self {
        Self { config, hub }
    }

    #[allow(clippy::unused_async)]
    pub async fn start(&self) {
        let config = self.config.clone();
        let hub = self.hub.clone();

        tokio::spawn(async move {
            let mut sys = System::new_all();
            let mut networks = Networks::new_with_refreshed_list();
            let mut disks = Disks::new_with_refreshed_list();
            let mut components = Components::new_with_refreshed_list();

            loop {
                let state = Self::gather_metrics(
                    &mut sys,
                    &mut networks,
                    &mut disks,
                    &mut components,
                    &config,
                );

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

    #[must_use]
    pub fn gather_metrics(
        sys: &mut System,
        networks: &mut Networks,
        disks: &mut Disks,
        components: &mut Components,
        config: &MetricsConfig,
    ) -> MetricsState {
        sys.refresh_cpu_usage();
        sys.refresh_memory();
        networks.refresh(true);
        disks.refresh(true);
        components.refresh(true);

        // CPU
        let nproc = f32::from(u16::try_from(sys.cpus().len()).unwrap_or(1));
        let global_cpu = sys.global_cpu_usage();

        let per_core_raw: Vec<f32> = sys.cpus().iter().map(sysinfo::Cpu::cpu_usage).collect();
        let (cpu_usage, per_core) =
            MetricsState::normalize_cpu_usage(config.cpu(), global_cpu, nproc, per_core_raw);

        // Network
        let mut network_tx: u64 = 0;
        let mut network_rx: u64 = 0;
        if config.network().is_some()
            && config.network() != Some(&crate::features::metrics::domain::NetworkMode::Disabled)
        {
            for (_interface_name, data) in networks.iter() {
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
                    crate::features::metrics::domain::MemoryBytes::new(disk.total_space()),
                    crate::features::metrics::domain::MemoryBytes::new(disk.available_space()),
                    crate::features::metrics::domain::MemoryBytes::new(
                        disk.total_space().saturating_sub(disk.available_space()),
                    ),
                ));
            }
        }

        // Temperature
        let mut temp = 0.0;
        if config.temperature().is_some()
            && config.temperature()
                != Some(&crate::features::metrics::domain::TemperatureMode::Disabled)
        {
            let mut count: u16 = 0;
            for component in components.iter() {
                if let Some(t) = component.temperature() {
                    temp += t;
                    count = count.saturating_add(1);
                }
            }
            if count > 0 {
                temp /= f32::from(count);
            }

            if config.temperature()
                == Some(&crate::features::metrics::domain::TemperatureMode::Fahrenheit)
            {
                temp = (temp * 9.0 / 5.0) + 32.0;
            }
        }

        MetricsState::new(crate::features::metrics::domain::CreateMetricsCommand::new(
            cpu_usage,
            per_core,
            crate::features::metrics::domain::MemoryBytes::new(sys.used_memory()),
            crate::features::metrics::domain::MemoryBytes::new(sys.total_memory()),
            crate::features::metrics::domain::MemoryBytes::new(sys.used_swap()),
            crate::features::metrics::domain::MemoryBytes::new(sys.total_swap()),
            disk_metrics,
            crate::features::metrics::domain::NetworkSpeed::new(network_tx),
            crate::features::metrics::domain::NetworkSpeed::new(network_rx),
            crate::features::metrics::domain::Temperature::new(temp),
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
        let mut sys = System::new_all();
        let mut networks = Networks::new_with_refreshed_list();
        let mut disks = Disks::new_with_refreshed_list();
        let mut components = Components::new_with_refreshed_list();

        let config: MetricsConfig = serde_json::from_value(serde_json::json!({
            "cpu": "percentage_0_100",
            "network": "tx_rx",
            "disk": "percentual",
            "temperature": "celsius",
            "update_interval_ms": 100
        }))
        .unwrap();

        let state = SysinfoAdapter::gather_metrics(
            &mut sys,
            &mut networks,
            &mut disks,
            &mut components,
            &config,
        );

        assert!(state.memory_total().value() > 0);
    }

    #[test]
    fn test_sysinfo_adapter_gather_metrics_all_disabled() {
        let mut sys = System::new_all();
        let mut networks = Networks::new_with_refreshed_list();
        let mut disks = Disks::new_with_refreshed_list();
        let mut components = Components::new_with_refreshed_list();

        let config: MetricsConfig = serde_json::from_value(serde_json::json!({
            "cpu": "disabled",
            "network": "disabled",
            "disk": "disabled",
            "temperature": "disabled",
            "update_interval_ms": 100
        }))
        .unwrap();

        let state = SysinfoAdapter::gather_metrics(
            &mut sys,
            &mut networks,
            &mut disks,
            &mut components,
            &config,
        );

        assert_eq!(state.network_tx().value(), 0);
        assert_eq!(state.network_rx().value(), 0);
        assert!(state.disks().is_empty());
        assert!(state.temperature().value().abs() < f32::EPSILON);
    }

    #[test]
    fn test_sysinfo_adapter_gather_metrics_fahrenheit() {
        let mut sys = System::new_all();
        let mut networks = Networks::new_with_refreshed_list();
        let mut disks = Disks::new_with_refreshed_list();
        let mut components = Components::new_with_refreshed_list();

        let config: MetricsConfig = serde_json::from_value(serde_json::json!({
            "temperature": "fahrenheit",
            "update_interval_ms": 100
        }))
        .unwrap();

        let state = SysinfoAdapter::gather_metrics(
            &mut sys,
            &mut networks,
            &mut disks,
            &mut components,
            &config,
        );
        // It might be 32.0 if no components, or > 32.0 if components exist
        assert!(state.temperature().value() >= 32.0 || state.temperature().value() == 0.0);
    }

    #[tokio::test]
    async fn test_sysinfo_adapter_start() {
        let config: MetricsConfig = serde_json::from_value(serde_json::json!({
            "update_interval_ms": 10
        }))
        .unwrap();
        let hub = Arc::new(SignalHub::new(Config::default()));
        let mut rx = hub.metrics_rx().clone();

        let adapter = SysinfoAdapter::new(config, hub);
        adapter.start().await;

        // Wait for first update
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let state = rx.borrow_and_update().clone();
        assert!(state.memory_total().value() > 0);
    }
}
