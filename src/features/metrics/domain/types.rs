//! Newtypes for single metric readings (CPU percentage, bytes, bytes transferred since the
//! previous sample, degrees in the configured scale) and per-disk usage.
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
