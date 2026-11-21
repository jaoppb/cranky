use crate::features::vdom::domain::VNode;
use serde::Deserialize;

/// Which side of its anchor node a script asked a popup to open on; parsed and diffed, but popup
/// placement does not read it yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnchorDirection {
    /// Above the anchor.
    Top,
    /// Below the anchor.
    Bottom,
    /// Left of the anchor.
    Left,
    /// Right of the anchor.
    Right,
    /// No side requested; the default.
    #[default]
    Auto,
}

pub use crate::shared::primitives::PopupOffset;

/// A popup a node opens: its content, the requested anchor side and an optional offset; a script
/// may give the content alone.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(from = "PopupSpecHelper")]
pub struct PopupSpec {
    content: Box<VNode>,
    anchor_direction: AnchorDirection,
    offset: Option<PopupOffset>,
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
    },
    Implicit(Box<VNode>),
}

impl From<PopupSpecHelper> for PopupSpec {
    fn from(helper: PopupSpecHelper) -> Self {
        match helper {
            PopupSpecHelper::Explicit {
                content,
                anchor_direction,
                offset,
            } => Self {
                content,
                anchor_direction,
                offset,
            },
            PopupSpecHelper::Implicit(content) => Self::new(content),
        }
    }
}

impl PopupSpec {
    #[must_use]
    pub(crate) const fn new(content: Box<VNode>) -> Self {
        Self {
            content,
            anchor_direction: AnchorDirection::Auto,
            offset: None,
        }
    }

    #[must_use]
    pub(crate) const fn with_anchor(mut self, anchor_direction: AnchorDirection) -> Self {
        self.anchor_direction = anchor_direction;
        self
    }

    #[must_use]
    pub(crate) const fn with_offset(mut self, offset: Option<PopupOffset>) -> Self {
        self.offset = offset;
        self
    }

    #[must_use]
    pub(crate) fn content(&self) -> &VNode {
        &self.content
    }

    #[must_use]
    pub(crate) const fn anchor_direction(&self) -> AnchorDirection {
        self.anchor_direction
    }

    #[must_use]
    pub(crate) const fn offset(&self) -> Option<PopupOffset> {
        self.offset
    }
}
