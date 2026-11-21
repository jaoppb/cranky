//! Value types the config domain is built from: fonts, borders, offsets, alignment, rendering
//! mode and scripting engine selection.
use serde::Deserialize;

/// Font family name handed to the text measurer and the canvas when shaping text.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct FontFamily(String);

impl FontFamily {
    /// Wraps a family name as written; it is not checked against the installed fonts here.
    #[must_use]
    pub const fn new(family: String) -> Self {
        Self(family)
    }

    #[must_use]
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

/// Font size handed to the text measurer and the canvas when shaping text.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct FontSize(f32);

impl FontSize {
    /// Wraps a size as given; no range check is applied.
    #[must_use]
    pub const fn new(size: f32) -> Self {
        Self(size)
    }

    #[must_use]
    pub(crate) const fn value(self) -> f32 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct BorderSize(f32);

impl BorderSize {
    #[must_use]
    pub(crate) const fn new(size: f32) -> Self {
        Self(size)
    }

    #[must_use]
    pub(crate) const fn value(self) -> f32 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub(crate) struct BorderRadius(f32);

impl BorderRadius {
    #[must_use]
    pub(crate) const fn new(radius: f32) -> Self {
        Self(radius)
    }

    #[must_use]
    pub(crate) const fn value(self) -> f32 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Deserialize, serde::Serialize)]
pub(crate) struct MarginOffset(i32);

impl MarginOffset {
    #[must_use]
    pub(crate) const fn new(offset: i32) -> Self {
        Self(offset)
    }

    #[must_use]
    pub(crate) const fn value(self) -> i32 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct PaddingOffset(u32);

impl PaddingOffset {
    #[must_use]
    pub(crate) const fn new(offset: u32) -> Self {
        Self(offset)
    }
}

/// Vertical placement requested for the bar's content by `vertical_alignment` (`top`, `center` or
/// `bottom`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VerticalAlignment {
    /// Aligned to the top edge.
    Top,
    /// Centred vertically; the default.
    #[default]
    Center,
    /// Aligned to the bottom edge.
    Bottom,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RenderingMode {
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
    pub(crate) const fn new_immediate(fps_limit: Option<u32>) -> Self {
        Self::Immediate { fps_limit }
    }

    #[must_use]
    pub(crate) const fn new_timebased(duration_ms: u64) -> Self {
        Self::Timebased { duration_ms }
    }
}

/// Name of a scripting engine as written in a module's `engine` key, such as `lua` or `rhai`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EngineId(String);

impl EngineId {
    #[must_use]
    pub(crate) fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    #[must_use]
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for EngineId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct FileExtension(String);

impl FileExtension {
    #[must_use]
    pub(crate) fn new(ext: impl Into<String>) -> Self {
        Self(ext.into())
    }

    #[must_use]
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for FileExtension {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Which scripting engine runs a module's script.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum EngineSelection {
    /// No `engine` key: try Lua first, then Rhai.
    #[default]
    Auto,
    /// The `engine` key pins this engine; no other is tried.
    Explicit(EngineId),
}

impl EngineSelection {
    /// True when no engine was pinned, so Lua and then Rhai are tried.
    #[cfg(test)]
    #[must_use]
    pub const fn is_auto(&self) -> bool {
        matches!(self, Self::Auto)
    }

    #[must_use]
    pub(crate) const fn as_explicit(&self) -> Option<&EngineId> {
        match self {
            Self::Explicit(id) => Some(id),
            Self::Auto => None,
        }
    }
}
