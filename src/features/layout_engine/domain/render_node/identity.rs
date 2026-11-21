use super::RenderNode;
use crate::features::vdom::domain::{NodeKey, NodePath};
use crate::shared::primitives::geometry::Rect;

impl RenderNode {
    #[must_use]
    pub(crate) const fn path(&self) -> &NodePath {
        match self {
            Self::Flex { path, .. }
            | Self::Grid { path, .. }
            | Self::Text { path, .. }
            | Self::Progress { path, .. }
            | Self::Rect { path, .. }
            | Self::Image { path, .. }
            | Self::Module { path, .. } => path,
        }
    }

    /// The reconciliation identity carried over from the `StyledNode` this
    /// was built from — see `StyledNode::node_key`. Pointer hit-testing uses
    /// this to remember hover/active/focus by identity rather than by
    /// position.
    #[must_use]
    pub(crate) const fn node_key(&self) -> Option<&NodeKey> {
        match self {
            Self::Flex { node_key, .. }
            | Self::Grid { node_key, .. }
            | Self::Text { node_key, .. }
            | Self::Progress { node_key, .. }
            | Self::Rect { node_key, .. }
            | Self::Image { node_key, .. }
            | Self::Module { node_key, .. } => node_key.as_ref(),
        }
    }

    #[must_use]
    pub(crate) const fn rect(&self) -> Rect {
        match self {
            Self::Flex { rect, .. }
            | Self::Grid { rect, .. }
            | Self::Text { rect, .. }
            | Self::Progress { rect, .. }
            | Self::Rect { rect, .. }
            | Self::Image { rect, .. }
            | Self::Module { rect, .. } => *rect,
        }
    }
}
