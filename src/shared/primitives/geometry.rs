//! Integer positions, sizes and rects, the logical and physical pixel units, and the monitor
//! scale that converts between them.
use serde::{Deserialize, Serialize};

/// A point in pixels; either coordinate may be negative.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Position {
    x: i32,
    y: i32,
}

impl Position {
    /// Builds a point from its horizontal and vertical coordinates.
    #[must_use]
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    #[must_use]
    pub(crate) const fn x(self) -> i32 {
        self.x
    }

    #[must_use]
    pub(crate) const fn y(self) -> i32 {
        self.y
    }
}

/// A width and height in whole pixels; the caller decides whether they are logical or physical.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Size {
    width: u32,
    height: u32,
}

impl Size {
    /// Builds a size; zero in either dimension is allowed.
    #[must_use]
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    /// Horizontal extent in pixels.
    #[must_use]
    pub const fn width(&self) -> u32 {
        self.width
    }

    /// Vertical extent in pixels.
    #[must_use]
    pub const fn height(&self) -> u32 {
        self.height
    }
}

/// An axis-aligned box: its top-left corner and its size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    position: Position,
    size: Size,
}

impl Rect {
    #[must_use]
    pub(crate) const fn new(position: Position, size: Size) -> Self {
        Self { position, size }
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) const fn position(&self) -> &Position {
        &self.position
    }

    #[must_use]
    pub(crate) const fn size(&self) -> &Size {
        &self.size
    }

    #[must_use]
    pub(crate) const fn x(&self) -> i32 {
        self.position.x()
    }

    #[must_use]
    pub(crate) const fn y(&self) -> i32 {
        self.position.y()
    }

    #[must_use]
    pub(crate) const fn width(&self) -> u32 {
        self.size.width()
    }

    #[must_use]
    pub(crate) const fn height(&self) -> u32 {
        self.size.height()
    }
}

/// A length in logical pixels, before the monitor's scale is applied.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct LogicalPx(f32);

impl LogicalPx {
    /// Wraps a length already expressed in logical pixels.
    #[must_use]
    pub const fn new(value: f32) -> Self {
        Self(value)
    }

    #[must_use]
    pub(crate) const fn value(self) -> f32 {
        self.0
    }

    #[must_use]
    pub(crate) fn apply_scale(self, scale: Scale) -> PhysicalPx {
        PhysicalPx::new(self.0 * scale.value())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub(crate) struct PhysicalPx(f32);

impl PhysicalPx {
    #[must_use]
    pub(crate) const fn new(value: f32) -> Self {
        Self(value)
    }

    #[must_use]
    pub(crate) const fn value(self) -> f32 {
        self.0
    }

    #[must_use]
    pub(crate) fn apply_inverse_scale(self, scale: Scale) -> LogicalPx {
        if scale.value() == 0.0 {
            LogicalPx::new(0.0)
        } else {
            LogicalPx::new(self.0 / scale.value())
        }
    }
}

/// A monitor's output scale factor, physical pixels per logical pixel, as reported by Wayland.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Scale(f32);

impl Scale {
    /// Wraps a factor as given; 0 makes logical conversions collapse to 0 rather than divide by
    /// zero.
    #[must_use]
    pub const fn new(value: f32) -> Self {
        Self(value)
    }

    #[must_use]
    pub(crate) const fn value(self) -> f32 {
        self.0
    }
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub(crate) struct BarWidth(u32);

#[cfg(test)]
impl BarWidth {
    #[must_use]
    pub(crate) const fn new(value: u32) -> Self {
        Self(value)
    }

    #[must_use]
    pub(crate) const fn value(self) -> u32 {
        self.0
    }
}

/// Height of the bar's layer surface in logical pixels, from `height` in `[root]` (30 by default).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct BarHeight(u32);

impl BarHeight {
    #[must_use]
    pub(crate) const fn new(value: u32) -> Self {
        Self(value)
    }

    /// The height in logical pixels.
    #[must_use]
    pub const fn value(&self) -> u32 {
        self.0
    }
}
