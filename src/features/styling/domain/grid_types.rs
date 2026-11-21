//! Value types for `display`, the grid properties and the supported pseudo-classes.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) enum DisplayMode {
    Flex,
    Grid,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub(crate) enum GridAutoFlow {
    #[default]
    Row,
    Column,
    RowDense,
    ColumnDense,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) enum GridTrack {
    #[serde(rename = "Px")]
    Pixels(f32),
    Percent(f32),
    #[serde(rename = "Fr")]
    Fraction(f32),
    Auto,
    MinContent,
    MaxContent,
    MinMax(Box<Self>, Box<Self>),
    Repeat(u16, Vec<Self>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub(crate) enum GridPlacement {
    #[default]
    Auto,
    Line(i16),
    Span(u16),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub(crate) struct GridLinePlacement {
    start: GridPlacement,
    end: GridPlacement,
}

impl GridLinePlacement {
    #[must_use]
    pub(crate) const fn new(start: GridPlacement, end: GridPlacement) -> Self {
        Self { start, end }
    }

    #[must_use]
    pub(crate) const fn start(self) -> GridPlacement {
        self.start
    }

    #[must_use]
    pub(crate) const fn end(self) -> GridPlacement {
        self.end
    }
}

/// A pseudo-class a selector can require; the vdom bridge derives the interactive ones from
/// pointer and focus state, the structural ones from the element's position among its siblings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PseudoClass {
    /// `:hover`: the pointer is over the element or one of its descendants.
    Hover,
    /// `:active`: a pointer button pressed on the element or a descendant is still held, with the
    /// pointer still on the surface.
    Active,
    /// `:focus` (also `:focus-visible`, `:focused`): the element a pointer button was last released
    /// over, or the bar root of the focused monitor; does not match ancestors.
    Focused,
    /// `:first-child`.
    FirstChild,
    /// `:last-child`.
    LastChild,
    /// `:only-child`.
    OnlyChild,
    /// `:nth-child(An+B)`, counting siblings from 1 at the start.
    NthChild {
        /// The `A` coefficient: the step between matching positions; 0 matches only position `B`,
        /// a negative step matches `B` and the positions before it.
        step_size: i32,
        /// The `B` term: the 1-based position the pattern starts from; may be zero or negative.
        offset: i32,
    },
    /// `:nth-last-child(An+B)`, counting siblings from 1 at the end.
    NthLastChild {
        /// The `A` coefficient: the step between matching positions; 0 matches only position `B`,
        /// a negative step matches `B` and the positions before it.
        step_size: i32,
        /// The `B` term: the 1-based position the pattern starts from; may be zero or negative.
        offset: i32,
    },
    /// `:empty`: a container with no children, or a text element with only whitespace.
    Empty,
}
