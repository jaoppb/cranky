use serde::{Deserialize, Serialize};

const TOP_FLAG: u8 = 1 << 0;
const BOTTOM_FLAG: u8 = 1 << 1;
const LEFT_FLAG: u8 = 1 << 2;
const RIGHT_FLAG: u8 = 1 << 3;

/// Vertical edge anchor for floating panels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VerticalAnchor {
    #[default]
    None,
    Top,
    Bottom,
    Both,
}

impl VerticalAnchor {
    #[must_use]
    pub const fn from_edges(top: bool, bottom: bool) -> Self {
        match (top, bottom) {
            (true, true) => Self::Both,
            (true, false) => Self::Top,
            (false, true) => Self::Bottom,
            (false, false) => Self::None,
        }
    }

    #[must_use]
    pub const fn to_flags(self) -> u8 {
        match self {
            Self::None => 0,
            Self::Top => TOP_FLAG,
            Self::Bottom => BOTTOM_FLAG,
            Self::Both => TOP_FLAG | BOTTOM_FLAG,
        }
    }
}

/// Horizontal edge anchor for floating panels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HorizontalAnchor {
    #[default]
    None,
    Left,
    Right,
    Both,
}

impl HorizontalAnchor {
    #[must_use]
    pub const fn from_edges(left: bool, right: bool) -> Self {
        match (left, right) {
            (true, true) => Self::Both,
            (true, false) => Self::Left,
            (false, true) => Self::Right,
            (false, false) => Self::None,
        }
    }

    #[must_use]
    pub const fn to_flags(self) -> u8 {
        match self {
            Self::None => 0,
            Self::Left => LEFT_FLAG,
            Self::Right => RIGHT_FLAG,
            Self::Both => LEFT_FLAG | RIGHT_FLAG,
        }
    }
}

/// Wayland Layer-Shell anchor configuration for floating panels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(from = "PanelAnchorHelper")]
pub struct PanelAnchor {
    flags: u8,
}

impl Serialize for PanelAnchor {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("PanelAnchor", 4)?;
        state.serialize_field("top", &self.top())?;
        state.serialize_field("bottom", &self.bottom())?;
        state.serialize_field("left", &self.left())?;
        state.serialize_field("right", &self.right())?;
        state.end()
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
enum PanelAnchorHelper {
    Flags {
        #[serde(default)]
        top: bool,
        #[serde(default)]
        bottom: bool,
        #[serde(default)]
        left: bool,
        #[serde(default)]
        right: bool,
    },
    List(Vec<String>),
    Single(String),
}

fn parse_anchor_str(s: &str) -> u8 {
    match s.to_lowercase().as_str() {
        "top" => TOP_FLAG,
        "bottom" => BOTTOM_FLAG,
        "left" => LEFT_FLAG,
        "right" => RIGHT_FLAG,
        "top_left" | "top-left" => TOP_FLAG | LEFT_FLAG,
        "top_right" | "top-right" => TOP_FLAG | RIGHT_FLAG,
        "bottom_left" | "bottom-left" => BOTTOM_FLAG | LEFT_FLAG,
        "bottom_right" | "bottom-right" => BOTTOM_FLAG | RIGHT_FLAG,
        _ => 0,
    }
}

impl From<PanelAnchorHelper> for PanelAnchor {
    fn from(helper: PanelAnchorHelper) -> Self {
        match helper {
            PanelAnchorHelper::Flags { top, bottom, left, right } => Self::new(
                VerticalAnchor::from_edges(top, bottom),
                HorizontalAnchor::from_edges(left, right),
            ),
            PanelAnchorHelper::List(list) => {
                Self::from_flags(list.iter().fold(0, |acc, item| acc | parse_anchor_str(item)))
            }
            PanelAnchorHelper::Single(s) => Self::from_flags(parse_anchor_str(&s)),
        }
    }
}

impl PanelAnchor {
    #[must_use]
    pub const fn from_flags(flags: u8) -> Self {
        Self { flags }
    }

    /// Creates a new `PanelAnchor` with explicit edge flags.
    #[must_use]
    pub const fn new(vertical: VerticalAnchor, horizontal: HorizontalAnchor) -> Self {
        Self {
            flags: vertical.to_flags() | horizontal.to_flags(),
        }
    }

    #[must_use]
    pub const fn none() -> Self {
        Self { flags: 0 }
    }

    #[must_use]
    pub const fn all() -> Self {
        Self { flags: TOP_FLAG | BOTTOM_FLAG | LEFT_FLAG | RIGHT_FLAG }
    }

    #[must_use]
    pub const fn top(&self) -> bool {
        (self.flags & TOP_FLAG) != 0
    }

    #[must_use]
    pub const fn bottom(&self) -> bool {
        (self.flags & BOTTOM_FLAG) != 0
    }

    #[must_use]
    pub const fn left(&self) -> bool {
        (self.flags & LEFT_FLAG) != 0
    }

    #[must_use]
    pub const fn right(&self) -> bool {
        (self.flags & RIGHT_FLAG) != 0
    }
}
