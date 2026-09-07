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

pub use crate::shared::primitives::PopupOffset;

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(from = "PopupSpecHelper")]
pub struct PopupSpec {
    content: Box<VNode>,
    anchor_direction: AnchorDirection,
    offset: Option<PopupOffset>,
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
        offset: Option<PopupOffset>,
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
    pub const fn new(content: Box<VNode>) -> Self {
        Self {
            content,
            anchor_direction: AnchorDirection::Auto,
            offset: None,
            dismiss_on_unfocus: true,
        }
    }

    #[must_use]
    pub const fn with_anchor(mut self, anchor_direction: AnchorDirection) -> Self {
        self.anchor_direction = anchor_direction;
        self
    }

    #[must_use]
    pub const fn with_offset(mut self, offset: Option<PopupOffset>) -> Self {
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
    pub const fn offset(&self) -> Option<PopupOffset> {
        self.offset
    }

    #[must_use]
    pub const fn dismiss_on_unfocus(&self) -> bool {
        self.dismiss_on_unfocus
    }
}
