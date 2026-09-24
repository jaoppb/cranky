mod interaction;
#[cfg(test)]
mod tests;

use super::popup::{StyledPanel, StyledPopup};
use super::styled_node::StyledNode;
use crate::features::styling::domain::{ComputedStyle, Orientation, ProgressValue};
use crate::features::vdom::domain::{ClickHandlers, NodeKey, NodePath, TextContent, UiAction};
use crate::shared::primitives::geometry::{Rect, Size};
use crate::shared::primitives::{BinaryData, ModuleKey};

#[derive(Debug, Clone, PartialEq)]
pub enum RenderNode {
    Flex {
        path: NodePath,
        node_key: Option<NodeKey>,
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
        node_key: Option<NodeKey>,
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
        node_key: Option<NodeKey>,
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
        node_key: Option<NodeKey>,
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
        node_key: Option<NodeKey>,
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
        node_key: Option<NodeKey>,
        rect: Rect,
        data: BinaryData,
        pixel_size: Size,
        tooltip: Option<Box<StyledNode>>,
        popup: Option<StyledPopup>,
        panel: Option<StyledPanel>,
    },
    Module {
        path: NodePath,
        node_key: Option<NodeKey>,
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

    /// The reconciliation identity carried over from the `StyledNode` this
    /// was built from — see `StyledNode::node_key`. Pointer hit-testing uses
    /// this to remember hover/active/focus by identity rather than by
    /// position.
    #[must_use]
    pub const fn node_key(&self) -> Option<&NodeKey> {
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
}
