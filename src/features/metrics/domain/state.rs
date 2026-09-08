use super::command::CreateMetricsCommand;
use super::config::{CpuMode, MetricsConfig};
use super::types::{CpuUsage, DiskMetric, MemoryBytes, NetworkSpeed, Temperature};
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MetricsState {
    pub(crate) cpu_usage: CpuUsage,
    pub(crate) per_core: Vec<CpuUsage>,
    pub(crate) memory_used: MemoryBytes,
    pub(crate) memory_total: MemoryBytes,
    pub(crate) swap_used: MemoryBytes,
    pub(crate) swap_total: MemoryBytes,
    pub(crate) disks: Vec<DiskMetric>,
    pub(crate) network_tx: NetworkSpeed,
    pub(crate) network_rx: NetworkSpeed,
    pub(crate) temperature: Temperature,
    pub(crate) config: MetricsConfig,
}

impl MetricsState {
    #[must_use]
    pub fn new(cmd: CreateMetricsCommand) -> Self {
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
    pub fn normalize_cpu_usage(
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

    #[must_use]
    pub const fn memory_total(&self) -> &MemoryBytes {
        &self.memory_total
    }

    #[must_use]
    pub fn disks(&self) -> &[DiskMetric] {
        &self.disks
    }

    #[must_use]
    pub const fn network_tx(&self) -> &NetworkSpeed {
        &self.network_tx
    }

    #[must_use]
    pub const fn network_rx(&self) -> &NetworkSpeed {
        &self.network_rx
    }

    #[must_use]
    pub const fn temperature(&self) -> &Temperature {
        &self.temperature
    }
}
