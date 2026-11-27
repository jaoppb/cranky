//! The metrics snapshot published to modules, with CPU usage scaled per the configured mode.
use super::command::{CpuMetrics, CreateMetricsCommand};
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

/// One raw CPU reading as sysinfo reports it, before the configured mode scales it: the
/// overall usage, the number of cores, and each core's usage.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CpuSample {
    global: f32,
    nproc: f32,
    per_core: Vec<f32>,
}

impl CpuSample {
    #[must_use]
    pub(crate) const fn new(global: f32, nproc: f32, per_core: Vec<f32>) -> Self {
        Self {
            global,
            nproc,
            per_core,
        }
    }

    /// Scales this reading as `mode` asks: as reported, multiplied by the core count, or
    /// zeroed when CPU usage is off.
    #[must_use]
    pub(crate) fn normalize(self, mode: &CpuMode) -> CpuMetrics {
        let Self {
            global,
            nproc,
            per_core,
        } = self;
        match mode {
            CpuMode::Percentage0to100 => CpuMetrics::new(
                CpuUsage::new(global),
                per_core.into_iter().map(CpuUsage::new).collect(),
            ),
            CpuMode::PercentageNproc => CpuMetrics::new(
                CpuUsage::new(global * nproc),
                per_core.into_iter().map(CpuUsage::new).collect(),
            ),
            CpuMode::Disabled => CpuMetrics::new(
                CpuUsage::new(0.0),
                per_core.into_iter().map(|_| CpuUsage::new(0.0)).collect(),
            ),
        }
    }
}

impl MetricsState {
    #[must_use]
    pub(crate) fn new(cmd: CreateMetricsCommand) -> Self {
        let (system, devices, config) = cmd.into_parts();
        let (cpu, memory) = system.into_parts();
        let (disks, network, temperature) = devices.into_parts();
        let (cpu_usage, per_core) = cpu.into_parts();
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
        DeviceMetrics, DiskName, DiskSpace, MemoryMetrics, MemoryUsage, MountPoint, NetworkMetrics,
        SystemMetrics, TemperatureMode,
    };

    /// A disk `sda` mounted on `/home`: 500 bytes, 120 available, 380 used.
    fn home_disk() -> DiskMetric {
        let space = DiskSpace::new(
            MemoryBytes::new(500),
            MemoryBytes::new(120),
            MemoryBytes::new(380),
        );
        DiskMetric::new(DiskName::new("sda"), MountPoint::new("/home"), space)
    }

    /// A command whose every reading holds a distinct value, carrying `disk` and `config`.
    fn command_with(disk: DiskMetric, config: MetricsConfig) -> CreateMetricsCommand {
        let cpu = CpuMetrics::new(
            CpuUsage::new(12.5),
            vec![CpuUsage::new(3.0), CpuUsage::new(7.0)],
        );
        let memory = MemoryMetrics::new(
            MemoryUsage::new(MemoryBytes::new(100), MemoryBytes::new(400)),
            MemoryUsage::new(MemoryBytes::new(30), MemoryBytes::new(90)),
        );
        let network = NetworkMetrics::new(NetworkSpeed::new(11), NetworkSpeed::new(22));
        let temperature = Some(Temperature::new(41.0));
        CreateMetricsCommand::new(
            SystemMetrics::new(cpu, memory),
            DeviceMetrics::new(vec![disk], network, temperature),
            config,
        )
    }

    #[test]
    fn test_new_moves_every_reading_into_its_own_field() {
        let config: MetricsConfig =
            serde_json::from_value(serde_json::json!({ "temperature": "fahrenheit" })).unwrap();
        let state = MetricsState::new(command_with(home_disk(), config.clone()));

        let expected = MetricsState {
            cpu_usage: CpuUsage::new(12.5),
            per_core: vec![CpuUsage::new(3.0), CpuUsage::new(7.0)],
            memory_used: MemoryBytes::new(100),
            memory_total: MemoryBytes::new(400),
            swap_used: MemoryBytes::new(30),
            swap_total: MemoryBytes::new(90),
            disks: vec![home_disk()],
            network_tx: NetworkSpeed::new(11),
            network_rx: NetworkSpeed::new(22),
            temperature: Some(Temperature::new(41.0)),
            config,
        };
        assert_eq!(state, expected);
        let temperature = state.config.temperature();
        assert_eq!(temperature, Some(&TemperatureMode::Fahrenheit));
    }
}
