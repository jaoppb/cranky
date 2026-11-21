use super::config::MetricsConfig;
use super::types::{CpuUsage, DiskMetric, MemoryBytes, NetworkSpeed, Temperature};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MemoryMetrics {
    used: MemoryBytes,
    total: MemoryBytes,
    swap_used: MemoryBytes,
    swap_total: MemoryBytes,
}

impl MemoryMetrics {
    #[must_use]
    pub(crate) const fn new(
        used: MemoryBytes,
        total: MemoryBytes,
        swap_used: MemoryBytes,
        swap_total: MemoryBytes,
    ) -> Self {
        Self {
            used,
            total,
            swap_used,
            swap_total,
        }
    }

    #[must_use]
    pub(crate) const fn used(&self) -> MemoryBytes {
        self.used
    }

    #[must_use]
    pub(crate) const fn total(&self) -> MemoryBytes {
        self.total
    }

    #[must_use]
    pub(crate) const fn swap_used(&self) -> MemoryBytes {
        self.swap_used
    }

    #[must_use]
    pub(crate) const fn swap_total(&self) -> MemoryBytes {
        self.swap_total
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NetworkMetrics {
    tx: NetworkSpeed,
    rx: NetworkSpeed,
}

impl NetworkMetrics {
    #[must_use]
    pub(crate) const fn new(tx: NetworkSpeed, rx: NetworkSpeed) -> Self {
        Self { tx, rx }
    }

    #[must_use]
    pub(crate) const fn tx(&self) -> NetworkSpeed {
        self.tx
    }

    #[must_use]
    pub(crate) const fn rx(&self) -> NetworkSpeed {
        self.rx
    }
}

pub(crate) struct CreateMetricsCommand {
    cpu_usage: CpuUsage,
    per_core: Vec<CpuUsage>,
    memory: MemoryMetrics,
    disks: Vec<DiskMetric>,
    network: NetworkMetrics,
    temperature: Temperature,
    config: MetricsConfig,
}

impl CreateMetricsCommand {
    #[must_use]
    pub(crate) const fn new(
        cpu_usage: CpuUsage,
        per_core: Vec<CpuUsage>,
        memory: MemoryMetrics,
        disks: Vec<DiskMetric>,
        network: NetworkMetrics,
        temperature: Temperature,
        config: MetricsConfig,
    ) -> Self {
        Self {
            cpu_usage,
            per_core,
            memory,
            disks,
            network,
            temperature,
            config,
        }
    }

    #[must_use]
    pub(crate) fn into_parts(
        self,
    ) -> (
        CpuUsage,
        Vec<CpuUsage>,
        MemoryMetrics,
        Vec<DiskMetric>,
        NetworkMetrics,
        Temperature,
        MetricsConfig,
    ) {
        (
            self.cpu_usage,
            self.per_core,
            self.memory,
            self.disks,
            self.network,
            self.temperature,
            self.config,
        )
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) const fn cpu_usage(&self) -> &CpuUsage {
        &self.cpu_usage
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) const fn memory(&self) -> &MemoryMetrics {
        &self.memory
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) const fn memory_used(&self) -> MemoryBytes {
        self.memory.used()
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) const fn network(&self) -> &NetworkMetrics {
        &self.network
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) const fn network_tx(&self) -> NetworkSpeed {
        self.network.tx()
    }
}
