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
    #[must_use]
    pub const fn new(val: u64) -> Self {
        Self(val)
    }

    #[must_use]
    pub const fn value(&self) -> u64 {
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

const fn default_update_interval() -> UpdateInterval {
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
    #[must_use]
    pub const fn cpu(&self) -> &CpuMode {
        &self.cpu
    }
    #[must_use]
    pub const fn memory(&self) -> &MemoryMode {
        &self.memory
    }
    #[must_use]
    pub const fn swap(&self) -> &MemoryMode {
        &self.swap
    }
    #[must_use]
    pub const fn network(&self) -> Option<&NetworkMode> {
        self.network.as_ref()
    }
    #[must_use]
    pub const fn temperature(&self) -> Option<&TemperatureMode> {
        self.temperature.as_ref()
    }
    #[must_use]
    pub const fn disk(&self) -> Option<&DiskMode> {
        self.disk.as_ref()
    }
    #[must_use]
    pub const fn update_interval_ms(&self) -> &UpdateInterval {
        &self.update_interval_ms
    }
}
