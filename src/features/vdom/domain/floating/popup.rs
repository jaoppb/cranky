use crate::features::vdom::domain::VNode;
use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnchorDirection {
    Top,
    Bottom,
    Left,
    Right,
    #[default]
    Auto,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, Deserialize)]
#[serde(from = "PopupOffsetHelper")]
pub struct PopupOffset {
    dx: i32,
    dy: i32,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum PopupOffsetHelper {
    Map {
        #[serde(alias = "x", default)]
        dx: i32,
        #[serde(alias = "y", default)]
        dy: i32,
    },
    Tuple((i32, i32)),
    List(Vec<i32>),
}

impl From<PopupOffsetHelper> for PopupOffset {
    fn from(helper: PopupOffsetHelper) -> Self {
        match helper {
            PopupOffsetHelper::Map { dx, dy } | PopupOffsetHelper::Tuple((dx, dy)) => {
                Self::new(dx, dy)
            }
            PopupOffsetHelper::List(list) => {
                let dx = list.first().copied().unwrap_or(0);
                let dy = list.get(1).copied().unwrap_or(0);
                Self::new(dx, dy)
            }
        }
    }
}

impl PopupOffset {
    #[must_use]
    pub const fn new(dx: i32, dy: i32) -> Self {
        Self { dx, dy }
    }

    #[must_use]
    pub const fn dx(&self) -> i32 {
        self.dx
    }

    #[must_use]
    pub const fn dy(&self) -> i32 {
        self.dy
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(from = "PopupSpecHelper")]
pub struct PopupSpec {
    content: Box<VNode>,
    anchor_direction: AnchorDirection,
    offset: PopupOffset,
    dismiss_on_unfocus: bool,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum PopupSpecHelper {
    Explicit {
        #[serde(alias = "node")]
        content: Box<VNode>,
        #[serde(alias = "anchor", default)]
        anchor_direction: AnchorDirection,
        #[serde(default)]
        offset: PopupOffset,
        #[serde(default = "default_true")]
        dismiss_on_unfocus: bool,
    },
    Implicit(Box<VNode>),
}

const fn default_true() -> bool {
    true
}

impl From<PopupSpecHelper> for PopupSpec {
    fn from(helper: PopupSpecHelper) -> Self {
        match helper {
            PopupSpecHelper::Explicit {
                content,
                anchor_direction,
                offset,
                dismiss_on_unfocus,
            } => Self {
                content,
                anchor_direction,
                offset,
                dismiss_on_unfocus,
            },
            PopupSpecHelper::Implicit(content) => Self::new(content),
        }
    }
}

impl PopupSpec {
    #[must_use]
    pub fn new(content: Box<VNode>) -> Self {
        Self {
            content,
            anchor_direction: AnchorDirection::Auto,
            offset: PopupOffset::default(),
            dismiss_on_unfocus: true,
        }
    }

    #[must_use]
    pub const fn with_anchor(mut self, anchor_direction: AnchorDirection) -> Self {
        self.anchor_direction = anchor_direction;
        self
    }

    #[must_use]
    pub const fn with_offset(mut self, offset: PopupOffset) -> Self {
        self.offset = offset;
        self
    }

    #[must_use]
    pub const fn with_dismiss_on_unfocus(mut self, dismiss_on_unfocus: bool) -> Self {
        self.dismiss_on_unfocus = dismiss_on_unfocus;
        self
    }

    #[must_use]
    pub fn content(&self) -> &VNode {
        &self.content
    }

    #[must_use]
    pub const fn anchor_direction(&self) -> AnchorDirection {
        self.anchor_direction
    }

    #[must_use]
    pub const fn offset(&self) -> PopupOffset {
        self.offset
    }

    #[must_use]
    pub const fn dismiss_on_unfocus(&self) -> bool {
        self.dismiss_on_unfocus
    }
}
