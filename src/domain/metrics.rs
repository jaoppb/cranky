use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum CpuMode {
    #[serde(rename = "percentage_0_100")]
    #[default]
    Percentage0to100,
    #[serde(rename = "percentage_nproc")]
    PercentageNproc,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MemoryMode {
    #[serde(rename = "absolute")]
    Absolute,
    #[serde(rename = "percentual")]
    #[default]
    Percentual,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum NetworkMode {
    #[serde(rename = "tx_rx")]
    TxRx,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TemperatureMode {
    #[serde(rename = "celsius")]
    Celsius,
    #[serde(rename = "fahrenheit")]
    Fahrenheit,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DiskMode {
    #[serde(rename = "absolute")]
    Absolute,
    #[serde(rename = "percentual")]
    #[default]
    Percentual,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateInterval(u64);

impl UpdateInterval {
    pub fn value(&self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MetricsConfig {
    #[serde(default)]
    cpu: CpuMode,
    #[serde(default)]
    memory: MemoryMode,
    #[serde(default)]
    swap: MemoryMode,
    network: Option<NetworkMode>,
    temperature: Option<TemperatureMode>,
    disk: Option<DiskMode>,
    #[serde(default = "default_update_interval")]
    update_interval_ms: UpdateInterval,
}

fn default_update_interval() -> UpdateInterval {
    UpdateInterval(1000)
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            cpu: CpuMode::default(),
            memory: MemoryMode::default(),
            swap: MemoryMode::default(),
            network: None,
            temperature: None,
            disk: None,
            update_interval_ms: default_update_interval(),
        }
    }
}

impl MetricsConfig {
    pub fn cpu(&self) -> &CpuMode {
        &self.cpu
    }
    pub fn network(&self) -> Option<&NetworkMode> {
        self.network.as_ref()
    }
    pub fn temperature(&self) -> Option<&TemperatureMode> {
        self.temperature.as_ref()
    }
    pub fn disk(&self) -> Option<&DiskMode> {
        self.disk.as_ref()
    }
    pub fn update_interval_ms(&self) -> &UpdateInterval {
        &self.update_interval_ms
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CpuUsage(f32);
impl CpuUsage {
    pub fn new(val: f32) -> Self {
        Self(val)
    }
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryBytes(u64);
impl MemoryBytes {
    pub fn new(val: u64) -> Self {
        Self(val)
    }
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkSpeed(u64);
impl NetworkSpeed {
    pub fn new(val: u64) -> Self {
        Self(val)
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Temperature(f32);
impl Temperature {
    pub fn new(val: f32) -> Self {
        Self(val)
    }
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiskName(String);
impl DiskName {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MountPoint(String);
impl MountPoint {
    pub fn new(mp: impl Into<String>) -> Self {
        Self(mp.into())
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DiskMetric {
    name: DiskName,
    mount_point: MountPoint,
    total_bytes: MemoryBytes,
    available_bytes: MemoryBytes,
    used_bytes: MemoryBytes,
}

impl DiskMetric {
    pub fn new(
        name: DiskName,
        mount_point: MountPoint,
        total_bytes: MemoryBytes,
        available_bytes: MemoryBytes,
        used_bytes: MemoryBytes,
    ) -> Self {
        Self {
            name,
            mount_point,
            total_bytes,
            available_bytes,
            used_bytes,
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MetricsState {
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
}


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

impl MetricsState {
    pub fn new(cmd: CreateMetricsCommand) -> Self {
        Self {
            cpu_usage: cmd.cpu_usage,
            per_core: cmd.per_core,
            memory_used: cmd.memory_used,
            memory_total: cmd.memory_total,
            swap_used: cmd.swap_used,
            swap_total: cmd.swap_total,
            disks: cmd.disks,
            network_tx: cmd.network_tx,
            network_rx: cmd.network_rx,
            temperature: cmd.temperature,
            config: cmd.config,
        }
    }

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
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_cpu_usage_0to100() {
        let global_cpu = 25.0; // 25% across 4 cores = 1 core fully loaded
        let nproc = 4.0;
        let per_core = vec![100.0, 0.0, 0.0, 0.0];

        let (norm_global, norm_per_core) = MetricsState::normalize_cpu_usage(
            &CpuMode::Percentage0to100,
            global_cpu,
            nproc,
            per_core.clone(),
        );
        assert_eq!(norm_global, CpuUsage::new(25.0));
        let expected_per_core: Vec<CpuUsage> = per_core.into_iter().map(CpuUsage::new).collect();
        assert_eq!(norm_per_core, expected_per_core);
    }

    #[test]
    fn test_normalize_cpu_usage_nproc() {
        let global_cpu = 25.0; // 25% across 4 cores = 1 core fully loaded
        let nproc = 4.0;
        let per_core = vec![100.0, 0.0, 0.0, 0.0];

        let (norm_global, norm_per_core) = MetricsState::normalize_cpu_usage(
            &CpuMode::PercentageNproc,
            global_cpu,
            nproc,
            per_core.clone(),
        );
        assert_eq!(norm_global, CpuUsage::new(100.0)); // 25.0 * 4 = 100.0%
        let expected_per_core: Vec<CpuUsage> = per_core.into_iter().map(CpuUsage::new).collect();
        assert_eq!(norm_per_core, expected_per_core);
    }
}
