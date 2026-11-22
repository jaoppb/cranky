//! The metrics snapshot published to modules, with CPU usage scaled per the configured mode.
use super::command::CreateMetricsCommand;
use super::config::{CpuMode, MetricsConfig};
use super::types::{CpuUsage, DiskMetric, MemoryBytes, NetworkSpeed, Temperature};
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct MetricsState {
    pub(crate) cpu_usage: CpuUsage,
    pub(crate) per_core: Vec<CpuUsage>,
    pub(crate) memory_used: MemoryBytes,
    pub(crate) memory_total: MemoryBytes,
    pub(crate) swap_used: MemoryBytes,
    pub(crate) swap_total: MemoryBytes,
    pub(crate) disks: Vec<DiskMetric>,
    pub(crate) network_tx: NetworkSpeed,
    pub(crate) network_rx: NetworkSpeed,
    /// The average sensor temperature, absent when nothing could be read or temperature is
    /// off; scripts then see no `temperature` key rather than a fake 0 °C.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) temperature: Option<Temperature>,
    pub(crate) config: MetricsConfig,
}

impl MetricsState {
    #[must_use]
    pub(crate) fn new(cmd: CreateMetricsCommand) -> Self {
        let (cpu_usage, per_core, memory, disks, network, temperature, config) = cmd.into_parts();
        Self {
            cpu_usage,
            per_core,
            memory_used: memory.used(),
            memory_total: memory.total(),
            swap_used: memory.swap_used(),
            swap_total: memory.swap_total(),
            disks,
            network_tx: network.tx(),
            network_rx: network.rx(),
            temperature,
            config,
        }
    }

    #[must_use]
    pub(crate) fn normalize_cpu_usage(
        mode: &CpuMode,
        global_cpu: f32,
        nproc: f32,
        per_core: Vec<f32>,
    ) -> (CpuUsage, Vec<CpuUsage>) {
        match mode {
            CpuMode::Percentage0to100 => (
                CpuUsage::new(global_cpu),
                per_core.into_iter().map(CpuUsage::new).collect(),
            ),
            CpuMode::PercentageNproc => (
                CpuUsage::new(global_cpu * nproc),
                per_core.into_iter().map(CpuUsage::new).collect(),
            ),
            CpuMode::Disabled => (
                CpuUsage::new(0.0),
                per_core.into_iter().map(|_| CpuUsage::new(0.0)).collect(),
            ),
        }
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) const fn memory_total(&self) -> &MemoryBytes {
        &self.memory_total
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) fn disks(&self) -> &[DiskMetric] {
        &self.disks
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) const fn network_tx(&self) -> &NetworkSpeed {
        &self.network_tx
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) const fn network_rx(&self) -> &NetworkSpeed {
        &self.network_rx
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) const fn temperature(&self) -> Option<&Temperature> {
        self.temperature.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::metrics::domain::{
        DiskName, MemoryMetrics, MountPoint, NetworkMetrics, TemperatureMode,
    };

    #[test]
    fn test_new_moves_every_reading_into_its_own_field() {
        let disk = DiskMetric::new(
            DiskName::new("sda"),
            MountPoint::new("/home"),
            MemoryBytes::new(500),
            MemoryBytes::new(120),
            MemoryBytes::new(380),
        );
        let config: MetricsConfig =
            serde_json::from_value(serde_json::json!({ "temperature": "fahrenheit" })).unwrap();
        let state = MetricsState::new(CreateMetricsCommand::new(
            CpuUsage::new(12.5),
            vec![CpuUsage::new(3.0), CpuUsage::new(7.0)],
            MemoryMetrics::new(
                MemoryBytes::new(100),
                MemoryBytes::new(400),
                MemoryBytes::new(30),
                MemoryBytes::new(90),
            ),
            vec![disk.clone()],
            NetworkMetrics::new(NetworkSpeed::new(11), NetworkSpeed::new(22)),
            Some(Temperature::new(41.0)),
            config.clone(),
        ));

        assert_eq!(state.cpu_usage, CpuUsage::new(12.5));
        assert_eq!(state.per_core, vec![CpuUsage::new(3.0), CpuUsage::new(7.0)]);
        assert_eq!(state.memory_used, MemoryBytes::new(100));
        assert_eq!(state.memory_total, MemoryBytes::new(400));
        assert_eq!(state.swap_used, MemoryBytes::new(30));
        assert_eq!(state.swap_total, MemoryBytes::new(90));
        assert_eq!(state.disks, vec![disk]);
        assert_eq!(state.network_tx, NetworkSpeed::new(11));
        assert_eq!(state.network_rx, NetworkSpeed::new(22));
        assert_eq!(state.temperature, Some(Temperature::new(41.0)));
        assert_eq!(
            state.config.temperature(),
            Some(&TemperatureMode::Fahrenheit)
        );
        assert_eq!(state.config, config);
    }
}
