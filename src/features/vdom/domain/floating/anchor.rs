//! The screen edges a panel is anchored to, as vertical and horizontal anchors packed into one
//! value.
use super::edge::{BOTTOM_FLAG, HorizontalAnchor, LEFT_FLAG, RIGHT_FLAG, TOP_FLAG, VerticalAnchor};
use serde::{Deserialize, Serialize};

/// Wayland Layer-Shell anchor configuration for floating panels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(from = "PanelAnchorHelper")]
pub(crate) struct PanelAnchor {
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

fn parse_anchor_str(name: &str) -> u8 {
    match name.to_lowercase().as_str() {
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

/// The union of the flags present, one per anchored edge.
fn union_of_flags(edges: &[Option<u8>]) -> u8 {
    edges.iter().flatten().fold(0, |acc, flag| acc | flag)
}

impl From<PanelAnchorHelper> for PanelAnchor {
    fn from(helper: PanelAnchorHelper) -> Self {
        match helper {
            PanelAnchorHelper::Flags {
                top,
                bottom,
                left,
                right,
            } => Self::from_flags(union_of_flags(&[
                top.then_some(TOP_FLAG),
                bottom.then_some(BOTTOM_FLAG),
                left.then_some(LEFT_FLAG),
                right.then_some(RIGHT_FLAG),
            ])),
            PanelAnchorHelper::List(list) => Self::from_flags(
                list.iter()
                    .fold(0, |acc, item| acc | parse_anchor_str(item)),
            ),
            PanelAnchorHelper::Single(name) => Self::from_flags(parse_anchor_str(&name)),
        }
    }
}

impl PanelAnchor {
    #[must_use]
    pub(crate) const fn from_flags(flags: u8) -> Self {
        Self { flags }
    }

    /// Creates a new `PanelAnchor` with explicit edge flags.
    #[must_use]
    pub(crate) const fn new(vertical: VerticalAnchor, horizontal: HorizontalAnchor) -> Self {
        Self {
            flags: vertical.to_flags() | horizontal.to_flags(),
        }
    }

    #[must_use]
    pub(crate) const fn none() -> Self {
        Self { flags: 0 }
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) const fn all() -> Self {
        Self {
            flags: TOP_FLAG | BOTTOM_FLAG | LEFT_FLAG | RIGHT_FLAG,
        }
    }

    #[must_use]
    pub(crate) const fn top(self) -> bool {
        (self.flags & TOP_FLAG) != 0
    }

    #[must_use]
    pub(crate) const fn bottom(self) -> bool {
        (self.flags & BOTTOM_FLAG) != 0
    }

    #[must_use]
    pub(crate) const fn left(self) -> bool {
        (self.flags & LEFT_FLAG) != 0
    }

    #[must_use]
    pub(crate) const fn right(self) -> bool {
        (self.flags & RIGHT_FLAG) != 0
    }
}
