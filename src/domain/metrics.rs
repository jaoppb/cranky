use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum CpuMode {
    #[serde(rename = "percentage_0_100")]
    #[default]
    Percentage0to100,
    #[serde(rename = "percentage_nproc")]
    PercentageNproc,
    #[serde(rename = "disabled")]
    Disabled,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MemoryMode {
    #[serde(rename = "absolute")]
    Absolute,
    #[serde(rename = "percentual")]
    #[default]
    Percentual,
    #[serde(rename = "disabled")]
    Disabled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum NetworkMode {
    #[serde(rename = "tx_rx")]
    TxRx,
    #[serde(rename = "disabled")]
    Disabled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TemperatureMode {
    #[serde(rename = "celsius")]
    Celsius,
    #[serde(rename = "fahrenheit")]
    Fahrenheit,
    #[serde(rename = "disabled")]
    Disabled,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DiskMode {
    #[serde(rename = "absolute")]
    Absolute,
    #[serde(rename = "percentual")]
    #[default]
    Percentual,
    #[serde(rename = "disabled")]
    Disabled,
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
    pub fn new(val: f32) -> Self { Self(val) }
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryBytes(u64);
impl MemoryBytes {
    pub fn new(val: u64) -> Self { Self(val) }
    #[cfg(test)]
    pub fn value(&self) -> u64 { self.0 }
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkSpeed(u64);
impl NetworkSpeed {
    pub fn new(val: u64) -> Self { Self(val) }
    #[cfg(test)]
    pub fn value(&self) -> u64 { self.0 }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Temperature(f32);
impl Temperature {
    pub fn new(val: f32) -> Self { Self(val) }
    #[cfg(test)]
    pub fn value(&self) -> f32 { self.0 }
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
            CpuMode::Disabled => (
                CpuUsage::new(0.0),
                per_core.into_iter().map(|_| CpuUsage::new(0.0)).collect(),
            ),
        }
    }
    
    #[cfg(test)]
    pub fn memory_total(&self) -> &MemoryBytes { &self.memory_total }
    #[cfg(test)]
    pub fn disks(&self) -> &[DiskMetric] { &self.disks }
    #[cfg(test)]
    pub fn network_tx(&self) -> &NetworkSpeed { &self.network_tx }
    #[cfg(test)]
    pub fn network_rx(&self) -> &NetworkSpeed { &self.network_rx }
    #[cfg(test)]
    pub fn temperature(&self) -> &Temperature { &self.temperature }
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

    #[test]
    fn test_normalize_cpu_usage_disabled() {
        let (norm_global, norm_per_core) = MetricsState::normalize_cpu_usage(
            &CpuMode::Disabled,
            25.0,
            4.0,
            vec![100.0, 0.0],
        );
        assert_eq!(norm_global, CpuUsage::new(0.0));
        assert_eq!(norm_per_core, vec![CpuUsage::new(0.0), CpuUsage::new(0.0)]);
    }

    #[test]
    fn test_metrics_config() {
        let config = MetricsConfig::default();
        assert_eq!(*config.cpu(), CpuMode::Percentage0to100);
        assert_eq!(config.network(), None);
        assert_eq!(config.temperature(), None);
        assert_eq!(config.disk(), None);
        assert_eq!(config.update_interval_ms().value(), 1000);
    }

    #[test]
    fn test_metrics_types() {
        assert_eq!(CpuUsage::new(42.0).0, 42.0);
        assert_eq!(MemoryBytes::new(1024).0, 1024);
        assert_eq!(NetworkSpeed::new(512).0, 512);
        assert_eq!(Temperature::new(60.0).0, 60.0);
        assert_eq!(DiskName::new("sda1"), DiskName("sda1".to_string()));
        assert_eq!(MountPoint::new("/mnt"), MountPoint("/mnt".to_string()));
    }

    #[test]
    fn test_disk_metric() {
        let dm = DiskMetric::new(
            DiskName::new("nvme0n1"),
            MountPoint::new("/"),
            MemoryBytes::new(100),
            MemoryBytes::new(20),
            MemoryBytes::new(80),
        );
        assert_eq!(dm.name, DiskName::new("nvme0n1"));
        assert_eq!(dm.mount_point, MountPoint::new("/"));
    }

    #[test]
    fn test_metrics_state_new() {
        let cmd = CreateMetricsCommand {
            cpu_usage: CpuUsage::new(10.0),
            per_core: vec![],
            memory_used: MemoryBytes::new(100),
            memory_total: MemoryBytes::new(200),
            swap_used: MemoryBytes::new(10),
            swap_total: MemoryBytes::new(20),
            disks: vec![],
            network_tx: NetworkSpeed::new(1),
            network_rx: NetworkSpeed::new(2),
            temperature: Temperature::new(40.0),
            config: MetricsConfig::default(),
        };
        let state = MetricsState::new(cmd);
        assert_eq!(state.cpu_usage, CpuUsage::new(10.0));
        assert_eq!(state.memory_used, MemoryBytes::new(100));
    }
}
