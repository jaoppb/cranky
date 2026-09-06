use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct FontFamily(String);

impl FontFamily {
    #[must_use]
    pub const fn new(family: String) -> Self {
        Self(family)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct FontSize(f32);

impl FontSize {
    #[must_use]
    pub const fn new(size: f32) -> Self {
        Self(size)
    }

    #[must_use]
    pub const fn value(&self) -> f32 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BorderSize(f32);

impl BorderSize {
    #[must_use]
    pub const fn new(size: f32) -> Self {
        Self(size)
    }

    #[must_use]
    pub const fn value(&self) -> f32 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct BorderRadius(f32);

impl BorderRadius {
    #[must_use]
    pub const fn new(radius: f32) -> Self {
        Self(radius)
    }

    #[must_use]
    pub const fn value(&self) -> f32 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MarginOffset(i32);

impl MarginOffset {
    #[must_use]
    pub const fn new(offset: i32) -> Self {
        Self(offset)
    }

    #[must_use]
    pub const fn value(&self) -> i32 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PaddingOffset(u32);

impl PaddingOffset {
    #[must_use]
    pub const fn new(offset: u32) -> Self {
        Self(offset)
    }

    #[must_use]
    pub const fn value(&self) -> u32 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VerticalAlignment {
    Top,
    #[default]
    Center,
    Bottom,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderingMode {
    Immediate { fps_limit: Option<u32> },
    Timebased { duration_ms: u64 },
}

impl Default for RenderingMode {
    fn default() -> Self {
        Self::Timebased { duration_ms: 100 }
    }
}

impl RenderingMode {
    #[must_use]
    pub const fn new_immediate(fps_limit: Option<u32>) -> Self {
        Self::Immediate { fps_limit }
    }

    #[must_use]
    pub const fn new_timebased(duration_ms: u64) -> Self {
        Self::Timebased { duration_ms }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EngineId(String);

impl EngineId {
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for EngineId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FileExtension(String);

impl FileExtension {
    #[must_use]
    pub fn new(ext: impl Into<String>) -> Self {
        Self(ext.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for FileExtension {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum EngineSelection {
    #[default]
    Auto,
    Explicit(EngineId),
}

impl EngineSelection {
    #[cfg(test)]
    #[must_use]
    pub const fn is_auto(&self) -> bool {
        matches!(self, Self::Auto)
    }

    #[must_use]
    pub const fn as_explicit(&self) -> Option<&EngineId> {
        match self {
            Self::Explicit(id) => Some(id),
            Self::Auto => None,
        }
    }
}
