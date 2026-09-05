use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DisplayMode {
    Flex,
    Grid,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum GridAutoFlow {
    #[default]
    Row,
    Column,
    RowDense,
    ColumnDense,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GridTrack {
    Px(f32),
    Percent(f32),
    Fr(f32),
    Auto,
    MinContent,
    MaxContent,
    MinMax(Box<Self>, Box<Self>),
    Repeat(u16, Vec<Self>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum GridPlacement {
    #[default]
    Auto,
    Line(i16),
    Span(u16),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct GridLinePlacement {
    start: GridPlacement,
    end: GridPlacement,
}

impl GridLinePlacement {
    #[must_use]
    pub const fn new(start: GridPlacement, end: GridPlacement) -> Self {
        Self { start, end }
    }

    #[must_use]
    pub const fn start(&self) -> GridPlacement {
        self.start
    }

    #[must_use]
    pub const fn end(&self) -> GridPlacement {
        self.end
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PseudoClass {
    Hover,
    Active,
    Focused,
    FirstChild,
    LastChild,
    OnlyChild,
    NthChild { a: i32, b: i32 },
    NthLastChild { a: i32, b: i32 },
    Empty,
}
