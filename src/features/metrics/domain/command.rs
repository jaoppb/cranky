//! The raw readings of one sampling pass, grouped before they become a `MetricsState`.
use super::config::MetricsConfig;
use super::types::{CpuUsage, DiskMetric, MemoryBytes, NetworkSpeed, Temperature};

/// How much of one kind of memory is in use: used bytes out of the total.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MemoryUsage {
    used: MemoryBytes,
    total: MemoryBytes,
}

impl MemoryUsage {
    #[must_use]
    pub(crate) const fn new(used: MemoryBytes, total: MemoryBytes) -> Self {
        Self { used, total }
    }
}

/// The memory readings of one pass: RAM and swap usage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MemoryMetrics {
    used: MemoryBytes,
    total: MemoryBytes,
    swap_used: MemoryBytes,
    swap_total: MemoryBytes,
}

impl MemoryMetrics {
    #[must_use]
    pub(crate) const fn new(ram: MemoryUsage, swap: MemoryUsage) -> Self {
        Self {
            used: ram.used,
            total: ram.total,
            swap_used: swap.used,
            swap_total: swap.total,
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

/// The CPU readings of one pass after normalization: overall usage and each core's.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CpuMetrics {
    usage: CpuUsage,
    per_core: Vec<CpuUsage>,
}

impl CpuMetrics {
    #[must_use]
    pub(crate) const fn new(usage: CpuUsage, per_core: Vec<CpuUsage>) -> Self {
        Self { usage, per_core }
    }

    #[must_use]
    pub(crate) fn into_parts(self) -> (CpuUsage, Vec<CpuUsage>) {
        (self.usage, self.per_core)
    }
}

/// What sysinfo's `System` reports in one pass: CPU and memory usage.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SystemMetrics {
    cpu: CpuMetrics,
    memory: MemoryMetrics,
}

impl SystemMetrics {
    #[must_use]
    pub(crate) const fn new(cpu: CpuMetrics, memory: MemoryMetrics) -> Self {
        Self { cpu, memory }
    }

    #[must_use]
    pub(crate) fn into_parts(self) -> (CpuMetrics, MemoryMetrics) {
        (self.cpu, self.memory)
    }
}

/// What sysinfo's device lists report in one pass: disk space (`Disks`), network traffic
/// (`Networks`) and the sensor temperature (`Components`).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DeviceMetrics {
    disks: Vec<DiskMetric>,
    network: NetworkMetrics,
    temperature: Option<Temperature>,
}

impl DeviceMetrics {
    #[must_use]
    pub(crate) const fn new(
        disks: Vec<DiskMetric>,
        network: NetworkMetrics,
        temperature: Option<Temperature>,
    ) -> Self {
        Self {
            disks,
            network,
            temperature,
        }
    }

    #[must_use]
    pub(crate) fn into_parts(self) -> (Vec<DiskMetric>, NetworkMetrics, Option<Temperature>) {
        (self.disks, self.network, self.temperature)
    }
}

pub(crate) struct CreateMetricsCommand {
    system: SystemMetrics,
    devices: DeviceMetrics,
    config: MetricsConfig,
}

impl CreateMetricsCommand {
    #[must_use]
    pub(crate) const fn new(
        system: SystemMetrics,
        devices: DeviceMetrics,
        config: MetricsConfig,
    ) -> Self {
        Self {
            system,
            devices,
            config,
        }
    }

    #[must_use]
    pub(crate) fn into_parts(self) -> (SystemMetrics, DeviceMetrics, MetricsConfig) {
        (self.system, self.devices, self.config)
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) const fn cpu_usage(&self) -> &CpuUsage {
        &self.system.cpu.usage
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) const fn memory(&self) -> &MemoryMetrics {
        &self.system.memory
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) const fn memory_used(&self) -> MemoryBytes {
        self.system.memory.used()
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) const fn network(&self) -> &NetworkMetrics {
        &self.devices.network
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) const fn network_tx(&self) -> NetworkSpeed {
        self.devices.network.tx()
    }
}
