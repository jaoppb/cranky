//! Newtypes for single metric readings (CPU percentage, bytes, bytes transferred since the
//! previous sample, degrees in the configured scale) and per-disk usage.
use super::config::TemperatureMode;
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct CpuUsage(f32);
impl CpuUsage {
    #[must_use]
    pub(crate) const fn new(val: f32) -> Self {
        Self(val)
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) const fn value(&self) -> f32 {
        self.0
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct MemoryBytes(u64);
impl MemoryBytes {
    #[must_use]
    pub(crate) const fn new(val: u64) -> Self {
        Self(val)
    }
    #[cfg(test)]
    #[must_use]
    pub(crate) const fn value(self) -> u64 {
        self.0
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct NetworkSpeed(u64);
impl NetworkSpeed {
    #[must_use]
    pub(crate) const fn new(val: u64) -> Self {
        Self(val)
    }
    #[cfg(test)]
    #[must_use]
    pub(crate) const fn value(self) -> u64 {
        self.0
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct Temperature(f32);
impl Temperature {
    #[cfg(test)]
    #[must_use]
    pub(crate) const fn new(val: f32) -> Self {
        Self(val)
    }

    /// Averages the sensor readings in degrees Celsius and expresses the result in `mode`'s
    /// scale, so the bar shows one figure for every component of the machine.
    ///
    /// Missing (`None`) and non-finite (NaN, infinite) readings are skipped, while negative
    /// ones are kept. Returns `None` when no reading is left or `mode` is `Disabled`, so an
    /// absent sensor never shows up as a real 0 °C.
    #[must_use]
    pub(crate) fn from_readings(
        readings: impl IntoIterator<Item = Option<f32>>,
        mode: &TemperatureMode,
    ) -> Option<Self> {
        if *mode == TemperatureMode::Disabled {
            return None;
        }
        let (sum, count) = readings
            .into_iter()
            .flatten()
            .filter(|reading| reading.is_finite())
            .fold((0.0_f32, 0.0_f32), |(sum, count), reading| {
                (sum + reading, count + 1.0)
            });
        if count == 0.0 {
            return None;
        }
        let celsius = sum / count;
        Some(Self(match mode {
            TemperatureMode::Fahrenheit => celsius * 9.0 / 5.0 + 32.0,
            TemperatureMode::Celsius | TemperatureMode::Disabled => celsius,
        }))
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) const fn value(&self) -> f32 {
        self.0
    }
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct DiskName(String);
impl DiskName {
    #[must_use]
    pub(crate) fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct MountPoint(String);
impl MountPoint {
    #[must_use]
    pub(crate) fn new(mount_point: impl Into<String>) -> Self {
        Self(mount_point.into())
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct DiskMetric {
    name: DiskName,
    mount_point: MountPoint,
    total_bytes: MemoryBytes,
    available_bytes: MemoryBytes,
    used_bytes: MemoryBytes,
}

impl DiskMetric {
    #[must_use]
    pub(crate) const fn new(
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

    #[cfg(test)]
    #[must_use]
    pub(crate) const fn name(&self) -> &DiskName {
        &self.name
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) const fn mount_point(&self) -> &MountPoint {
        &self.mount_point
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_disk_metric_keeps_total_available_and_used_apart() {
        let disk = DiskMetric::new(
            DiskName::new("nvme1"),
            MountPoint::new("/data"),
            MemoryBytes::new(1000),
            MemoryBytes::new(250),
            MemoryBytes::new(750),
        );
        assert_eq!(
            serde_json::to_value(disk).unwrap(),
            serde_json::json!({
                "name": "nvme1",
                "mount_point": "/data",
                "total_bytes": 1000,
                "available_bytes": 250,
                "used_bytes": 750,
            })
        );
    }
}
