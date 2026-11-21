use super::StyledNode;
#[cfg(test)]
use crate::features::layout_engine::domain::popup::{StyledPanel, StyledPopup};
use crate::features::styling::domain::ComputedStyle;
use crate::features::vdom::domain::{NodeKey, NodePath};

impl StyledNode {
    /// Positional path of this node within its surface's tree, whatever the variant.
    #[must_use]
    pub const fn path(&self) -> &NodePath {
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

    /// Reconciliation identity set by the widget script (`key = "..."` in
    /// the DSL), distinct from `path` (positional) and from `Module`'s own
    /// `key: ModuleKey`. `None` unless the widget opted in.
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
    pub(crate) const fn style(&self) -> &ComputedStyle {
        match self {
            Self::Flex { style, .. }
            | Self::Grid { style, .. }
            | Self::Text { style, .. }
            | Self::Progress { style, .. }
            | Self::Rect { style, .. }
            | Self::Image { style, .. }
            | Self::Module { style, .. } => style,
        }
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) fn tooltip(&self) -> Option<&Self> {
        match self {
            Self::Flex { tooltip, .. }
            | Self::Grid { tooltip, .. }
            | Self::Text { tooltip, .. }
            | Self::Progress { tooltip, .. }
            | Self::Rect { tooltip, .. }
            | Self::Image { tooltip, .. }
            | Self::Module { tooltip, .. } => tooltip.as_deref(),
        }
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) const fn popup(&self) -> Option<&StyledPopup> {
        match self {
            Self::Flex { popup, .. }
            | Self::Grid { popup, .. }
            | Self::Text { popup, .. }
            | Self::Progress { popup, .. }
            | Self::Rect { popup, .. }
            | Self::Image { popup, .. }
            | Self::Module { popup, .. } => popup.as_ref(),
        }
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) const fn panel(&self) -> Option<&StyledPanel> {
        match self {
            Self::Flex { panel, .. }
            | Self::Grid { panel, .. }
            | Self::Text { panel, .. }
            | Self::Progress { panel, .. }
            | Self::Rect { panel, .. }
            | Self::Image { panel, .. }
            | Self::Module { panel, .. } => panel.as_ref(),
        }
    }
}
