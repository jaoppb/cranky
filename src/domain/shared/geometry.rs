use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    x: i32,
    y: i32,
}

impl Position {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    pub fn x(&self) -> i32 {
        self.x
    }

    pub fn y(&self) -> i32 {
        self.y
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Size {
    width: u32,
    height: u32,
}

impl Size {
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    position: Position,
    size: Size,
}

impl Rect {
    pub fn new(position: Position, size: Size) -> Self {
        Self { position, size }
    }

    pub fn position(&self) -> &Position {
        &self.position
    }

    pub fn size(&self) -> &Size {
        &self.size
    }

    pub fn x(&self) -> i32 {
        self.position.x()
    }

    pub fn y(&self) -> i32 {
        self.position.y()
    }

    pub fn width(&self) -> u32 {
        self.size.width()
    }

    pub fn height(&self) -> u32 {
        self.size.height()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point64 {
    x: f64,
    y: f64,
}

impl Point64 {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    pub fn x(&self) -> f64 {
        self.x
    }

    pub fn y(&self) -> f64 {
        self.y
    }
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct LogicalPx(f32);

impl LogicalPx {
    pub fn new(value: f32) -> Self { Self(value) }
    pub fn value(&self) -> f32 { self.0 }
    
    pub fn apply_scale(&self, scale: &Scale) -> PhysicalPx {
        PhysicalPx::new(self.0 * scale.value())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct PhysicalPx(f32);

impl PhysicalPx {
    pub fn new(value: f32) -> Self { Self(value) }
    pub fn value(&self) -> f32 { self.0 }
    
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
    pub fn new(value: f32) -> Self {
        Self(value)
    }

    pub fn value(&self) -> f32 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct BarWidth(u32);

impl BarWidth {
    pub fn new(value: u32) -> Self { Self(value) }
    pub fn value(&self) -> u32 { self.0 }
}
