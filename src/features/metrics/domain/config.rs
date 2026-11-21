//! The `[metrics]` table of `config.toml`: which readings to take, in which unit, and how
//! often.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub(crate) enum CpuMode {
    #[serde(rename = "percentage_0_100")]
    #[default]
    Percentage0to100,
    #[serde(rename = "percentage_nproc")]
    PercentageNproc,
    #[serde(rename = "disabled")]
    Disabled,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) enum MemoryMode {
    #[serde(rename = "absolute")]
    Absolute,
    #[serde(rename = "percentual")]
    #[default]
    Percentual,
    #[serde(rename = "disabled")]
    Disabled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) enum NetworkMode {
    #[serde(rename = "tx_rx")]
    TxRx,
    #[serde(rename = "disabled")]
    Disabled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) enum TemperatureMode {
    #[serde(rename = "celsius")]
    Celsius,
    #[serde(rename = "fahrenheit")]
    Fahrenheit,
    #[serde(rename = "disabled")]
    Disabled,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) enum DiskMode {
    #[serde(rename = "absolute")]
    Absolute,
    #[serde(rename = "percentual")]
    #[default]
    Percentual,
    #[serde(rename = "disabled")]
    Disabled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct UpdateInterval(u64);

impl UpdateInterval {
    #[must_use]
    pub(crate) const fn value(&self) -> u64 {
        self.0
    }
}

/// The `[metrics]` table: which readings the sampler takes and how often.
///
/// It also holds the unit each reading is shown in (the widget scripts apply the memory, swap and
/// disk modes) and the sampling interval in milliseconds (1000 by default). Network, temperature
/// and disk are off unless set.
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
    pub(crate) const fn cpu(&self) -> &CpuMode {
        &self.cpu
    }
    #[must_use]
    pub(crate) const fn network(&self) -> Option<&NetworkMode> {
        self.network.as_ref()
    }
    #[must_use]
    pub(crate) const fn temperature(&self) -> Option<&TemperatureMode> {
        self.temperature.as_ref()
    }
    #[must_use]
    pub(crate) const fn disk(&self) -> Option<&DiskMode> {
        self.disk.as_ref()
    }
    #[must_use]
    pub(crate) const fn update_interval_ms(&self) -> &UpdateInterval {
        &self.update_interval_ms
    }
}
