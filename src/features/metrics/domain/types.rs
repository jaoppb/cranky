use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CpuUsage(f32);
impl CpuUsage {
    #[must_use]
    pub const fn new(val: f32) -> Self {
        Self(val)
    }

    #[must_use]
    pub const fn value(&self) -> f32 {
        self.0
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryBytes(u64);
impl MemoryBytes {
    #[must_use]
    pub const fn new(val: u64) -> Self {
        Self(val)
    }
    #[must_use]
    pub const fn value(&self) -> u64 {
        self.0
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkSpeed(u64);
impl NetworkSpeed {
    #[must_use]
    pub const fn new(val: u64) -> Self {
        Self(val)
    }
    #[must_use]
    pub const fn value(&self) -> u64 {
        self.0
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Temperature(f32);
impl Temperature {
    #[must_use]
    pub const fn new(val: f32) -> Self {
        Self(val)
    }
    #[must_use]
    pub const fn value(&self) -> f32 {
        self.0
    }
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiskName(String);
impl DiskName {
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MountPoint(String);
impl MountPoint {
    #[must_use]
    pub fn new(mp: impl Into<String>) -> Self {
        Self(mp.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiskMetric {
    name: DiskName,
    mount_point: MountPoint,
    total_bytes: MemoryBytes,
    available_bytes: MemoryBytes,
    used_bytes: MemoryBytes,
}

impl DiskMetric {
    #[must_use]
    pub const fn new(
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

    #[must_use]
    pub const fn name(&self) -> &DiskName {
        &self.name
    }

    #[must_use]
    pub const fn mount_point(&self) -> &MountPoint {
        &self.mount_point
    }

    #[must_use]
    pub const fn total_bytes(&self) -> &MemoryBytes {
        &self.total_bytes
    }

    #[must_use]
    pub const fn available_bytes(&self) -> &MemoryBytes {
        &self.available_bytes
    }

    #[must_use]
    pub const fn used_bytes(&self) -> &MemoryBytes {
        &self.used_bytes
    }
}
