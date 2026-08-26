use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Position {
    x: i32,
    y: i32,
}

impl Position {
    #[must_use]
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    #[must_use]
    pub const fn x(&self) -> i32 {
        self.x
    }

    #[must_use]
    pub const fn y(&self) -> i32 {
        self.y
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Size {
    width: u32,
    height: u32,
}

impl Size {
    #[must_use]
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    #[must_use]
    pub const fn width(&self) -> u32 {
        self.width
    }

    #[must_use]
    pub const fn height(&self) -> u32 {
        self.height
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    position: Position,
    size: Size,
}

impl Rect {
    #[must_use]
    pub const fn new(position: Position, size: Size) -> Self {
        Self { position, size }
    }

    #[cfg(test)]
    #[must_use]
    pub const fn position(&self) -> &Position {
        &self.position
    }

    #[must_use]
    pub const fn size(&self) -> &Size {
        &self.size
    }

    #[must_use]
    pub const fn x(&self) -> i32 {
        self.position.x()
    }

    #[must_use]
    pub const fn y(&self) -> i32 {
        self.position.y()
    }

    #[must_use]
    pub const fn width(&self) -> u32 {
        self.size.width()
    }

    #[must_use]
    pub const fn height(&self) -> u32 {
        self.size.height()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct LogicalPx(f32);

impl LogicalPx {
    #[must_use]
    pub const fn new(value: f32) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn value(&self) -> f32 {
        self.0
    }

    #[must_use]
    pub fn apply_scale(&self, scale: &Scale) -> PhysicalPx {
        PhysicalPx::new(self.0 * scale.value())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct PhysicalPx(f32);

impl PhysicalPx {
    #[must_use]
    pub const fn new(value: f32) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn value(&self) -> f32 {
        self.0
    }

    #[must_use]
    pub fn apply_inverse_scale(&self, scale: &Scale) -> LogicalPx {
        if scale.value() == 0.0 {
            LogicalPx::new(0.0)
        } else {
            LogicalPx::new(self.0 / scale.value())
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Scale(f32);

impl Scale {
    #[must_use]
    pub const fn new(value: f32) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn value(&self) -> f32 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct BarWidth(u32);

impl BarWidth {
    #[must_use]
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn value(&self) -> u32 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct BarHeight(u32);

impl BarHeight {
    #[must_use]
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn value(&self) -> u32 {
        self.0
    }
}
