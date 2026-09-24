use super::RenderNode;
use crate::features::layout_engine::domain::styled_node::StyledNode;
use crate::features::layout_engine::domain::popup::{StyledPanel, StyledPopup};
use crate::features::vdom::domain::{ClickHandlers, UiAction};

impl RenderNode {
    #[must_use]
    pub const fn on_click(&self) -> Option<&ClickHandlers> {
        match self {
            Self::Text { on_click, .. }
            | Self::Flex { on_click, .. }
            | Self::Grid { on_click, .. }
            | Self::Progress { on_click, .. }
            | Self::Rect { on_click, .. }
            | Self::Module { on_click, .. } => on_click.as_ref(),
            Self::Image { .. } => None,
        }
    }

    #[must_use]
    pub const fn on_hover(&self) -> Option<&UiAction> {
        match self {
            Self::Text { on_hover, .. }
            | Self::Flex { on_hover, .. }
            | Self::Grid { on_hover, .. }
            | Self::Progress { on_hover, .. }
            | Self::Rect { on_hover, .. }
            | Self::Module { on_hover, .. } => on_hover.as_ref(),
            Self::Image { .. } => None,
        }
    }

    #[must_use]
    pub fn tooltip(&self) -> Option<&StyledNode> {
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

    #[must_use]
    pub const fn popup(&self) -> Option<&StyledPopup> {
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

    #[must_use]
    pub const fn panel(&self) -> Option<&StyledPanel> {
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
