use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CpuMode {
    #[serde(rename = "percentage_0_100")]
    Percentage0to100,
    #[serde(rename = "percentage_nproc")]
    PercentageNproc,
}

impl Default for CpuMode {
    fn default() -> Self {
        Self::Percentage0to100
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MemoryMode {
    #[serde(rename = "absolute")]
    Absolute,
    #[serde(rename = "percentual")]
    Percentual,
}

impl Default for MemoryMode {
    fn default() -> Self {
        Self::Percentual
    }
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DiskMode {
    #[serde(rename = "absolute")]
    Absolute,
    #[serde(rename = "percentual")]
    Percentual,
}

impl Default for DiskMode {
    fn default() -> Self {
        Self::Percentual
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MetricsConfig {
    #[serde(default)]
    pub cpu: CpuMode,
    #[serde(default)]
    pub memory: MemoryMode,
    #[serde(default)]
    pub swap: MemoryMode,
    pub network: Option<NetworkMode>,
    pub temperature: Option<TemperatureMode>,
    pub disk: Option<DiskMode>,
    #[serde(default = "default_update_interval")]
    pub update_interval_ms: u64,
}

fn default_update_interval() -> u64 {
    1000
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

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct DiskMetric {
    pub name: String,
    pub mount_point: String,
    pub total_bytes: u64,
    pub available_bytes: u64,
    pub used_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct MetricsState {
    /// Global CPU usage (format depends on CpuMode)
    pub cpu_usage: f32,
    /// Per-core CPU usage (format depends on CpuMode)
    pub per_core: Vec<f32>,
    /// Memory used in bytes
    pub memory_used: u64,
    /// Total memory in bytes
    pub memory_total: u64,
    /// Swap used in bytes
    pub swap_used: u64,
    /// Total swap in bytes
    pub swap_total: u64,
    /// Disks
    pub disks: Vec<DiskMetric>,
    /// Total network TX speed (bytes/sec)
    pub network_tx: u64,
    /// Total network RX speed (bytes/sec)
    pub network_rx: u64,
    /// Average global temperature (if mode is set)
    pub temperature: f32,
    /// Passed back to the module to know what format the user configured
    pub config: MetricsConfig,
}

impl MetricsState {
    pub fn normalize_cpu_usage(mode: &CpuMode, global_cpu: f32, nproc: f32, per_core: Vec<f32>) -> (f32, Vec<f32>) {
        match mode {
            CpuMode::Percentage0to100 => {
                (global_cpu, per_core)
            }
            CpuMode::PercentageNproc => {
                (global_cpu * nproc, per_core)
            }
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
        
        let (norm_global, norm_per_core) = MetricsState::normalize_cpu_usage(&CpuMode::Percentage0to100, global_cpu, nproc, per_core.clone());
        assert_eq!(norm_global, 25.0);
        assert_eq!(norm_per_core, per_core);
    }

    #[test]
    fn test_normalize_cpu_usage_nproc() {
        let global_cpu = 25.0; // 25% across 4 cores = 1 core fully loaded
        let nproc = 4.0;
        let per_core = vec![100.0, 0.0, 0.0, 0.0];
        
        let (norm_global, norm_per_core) = MetricsState::normalize_cpu_usage(&CpuMode::PercentageNproc, global_cpu, nproc, per_core.clone());
        assert_eq!(norm_global, 100.0); // 25.0 * 4 = 100.0%
        assert_eq!(norm_per_core, per_core);
    }
}
