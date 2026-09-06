use super::config::MetricsConfig;
use super::types::{CpuUsage, DiskMetric, MemoryBytes, NetworkSpeed, Temperature};

pub struct CreateMetricsCommand {
    pub cpu_usage: CpuUsage,
    pub per_core: Vec<CpuUsage>,
    pub memory_used: MemoryBytes,
    pub memory_total: MemoryBytes,
    pub swap_used: MemoryBytes,
    pub swap_total: MemoryBytes,
    pub disks: Vec<DiskMetric>,
    pub network_tx: NetworkSpeed,
    pub network_rx: NetworkSpeed,
    pub temperature: Temperature,
    pub config: MetricsConfig,
}

impl CreateMetricsCommand {
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub const fn new(
        cpu_usage: CpuUsage,
        per_core: Vec<CpuUsage>,
        memory_used: MemoryBytes,
        memory_total: MemoryBytes,
        swap_used: MemoryBytes,
        swap_total: MemoryBytes,
        disks: Vec<DiskMetric>,
        network_tx: NetworkSpeed,
        network_rx: NetworkSpeed,
        temperature: Temperature,
        config: MetricsConfig,
    ) -> Self {
        Self {
            cpu_usage,
            per_core,
            memory_used,
            memory_total,
            swap_used,
            swap_total,
            disks,
            network_tx,
            network_rx,
            temperature,
            config,
        }
    }

    #[must_use]
    pub const fn cpu_usage(&self) -> &CpuUsage {
        &self.cpu_usage
    }

    #[must_use]
    pub fn per_core(&self) -> &[CpuUsage] {
        &self.per_core
    }

    #[must_use]
    pub const fn memory_used(&self) -> &MemoryBytes {
        &self.memory_used
    }

    #[must_use]
    pub const fn memory_total(&self) -> &MemoryBytes {
        &self.memory_total
    }

    #[must_use]
    pub const fn swap_used(&self) -> &MemoryBytes {
        &self.swap_used
    }

    #[must_use]
    pub const fn swap_total(&self) -> &MemoryBytes {
        &self.swap_total
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

    #[must_use]
    pub const fn config(&self) -> &MetricsConfig {
        &self.config
    }
}
