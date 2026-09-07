use super::popup::{StyledPanel, StyledPopup};
use super::styled_node::StyledNode;
use crate::features::styling::domain::{ComputedStyle, Orientation, ProgressValue};
use crate::features::vdom::domain::{ClickHandlers, NodePath, TextContent, UiAction};
use crate::shared::primitives::geometry::{Rect, Size};
use crate::shared::primitives::{BinaryData, ModuleKey};

#[derive(Debug, Clone, PartialEq)]
pub enum RenderNode {
    Flex {
        path: NodePath,
        rect: Rect,
        children: Vec<Self>,
        style: ComputedStyle,
        on_click: Option<ClickHandlers>,
        on_hover: Option<UiAction>,
        tooltip: Option<Box<StyledNode>>,
        popup: Option<StyledPopup>,
        panel: Option<StyledPanel>,
    },
    Grid {
        path: NodePath,
        rect: Rect,
        children: Vec<Self>,
        style: ComputedStyle,
        on_click: Option<ClickHandlers>,
        on_hover: Option<UiAction>,
        tooltip: Option<Box<StyledNode>>,
        popup: Option<StyledPopup>,
        panel: Option<StyledPanel>,
    },
    Text {
        path: NodePath,
        rect: Rect,
        text: TextContent,
        style: ComputedStyle,
        on_click: Option<ClickHandlers>,
        on_hover: Option<UiAction>,
        tooltip: Option<Box<StyledNode>>,
        popup: Option<StyledPopup>,
        panel: Option<StyledPanel>,
    },
    Progress {
        path: NodePath,
        rect: Rect,
        value: ProgressValue,
        orientation: Orientation,
        style: ComputedStyle,
        on_click: Option<ClickHandlers>,
        on_hover: Option<UiAction>,
        tooltip: Option<Box<StyledNode>>,
        popup: Option<StyledPopup>,
        panel: Option<StyledPanel>,
    },
    Rect {
        path: NodePath,
        rect: Rect,
        style: ComputedStyle,
        on_click: Option<ClickHandlers>,
        on_hover: Option<UiAction>,
        tooltip: Option<Box<StyledNode>>,
        popup: Option<StyledPopup>,
        panel: Option<StyledPanel>,
    },
    Image {
        path: NodePath,
        rect: Rect,
        data: BinaryData,
        pixel_size: Size,
        tooltip: Option<Box<StyledNode>>,
        popup: Option<StyledPopup>,
        panel: Option<StyledPanel>,
    },
    Module {
        path: NodePath,
        rect: Rect,
        key: ModuleKey,
        style: ComputedStyle,
        on_click: Option<ClickHandlers>,
        on_hover: Option<UiAction>,
        tooltip: Option<Box<StyledNode>>,
        popup: Option<StyledPopup>,
        panel: Option<StyledPanel>,
    },
}

impl RenderNode {
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

    #[must_use]
    pub const fn rect(&self) -> Rect {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::primitives::geometry::Position;

    #[test]
    fn test_render_node_accessors() {
        let rect = Rect::new(Position::new(0, 0), Size::new(10, 10));
        let node = RenderNode::Rect {
            path: NodePath::root(),
            rect,
            style: ComputedStyle::default(),
            on_click: None,
            on_hover: None,
            tooltip: None,
            popup: None,
            panel: None,
        };
        assert_eq!(node.rect(), rect);
        assert_eq!(node.on_click(), None);
        assert_eq!(node.on_hover(), None);
        assert_eq!(node.popup(), None);
        assert_eq!(node.panel(), None);
    }
}
