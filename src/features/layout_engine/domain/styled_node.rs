use super::popup::{StyledPanel, StyledPopup};
use crate::features::styling::domain::{ComputedStyle, Orientation, ProgressValue};
use crate::features::vdom::domain::{ClickHandlers, NodeKey, NodePath, TextContent, UiAction};
use crate::shared::primitives::geometry::Size;
use crate::shared::primitives::{BinaryData, ModuleKey, ModuleOptions};

#[derive(Debug, Clone, PartialEq)]
pub enum StyledNode {
    Flex {
        path: NodePath,
        node_key: Option<NodeKey>,
        children: Vec<Self>,
        style: ComputedStyle,
        on_click: Option<ClickHandlers>,
        on_hover: Option<UiAction>,
        tooltip: Option<Box<Self>>,
        popup: Option<StyledPopup>,
        panel: Option<StyledPanel>,
    },
    Grid {
        path: NodePath,
        node_key: Option<NodeKey>,
        children: Vec<Self>,
        style: ComputedStyle,
        on_click: Option<ClickHandlers>,
        on_hover: Option<UiAction>,
        tooltip: Option<Box<Self>>,
        popup: Option<StyledPopup>,
        panel: Option<StyledPanel>,
    },
    Text {
        path: NodePath,
        node_key: Option<NodeKey>,
        text: TextContent,
        style: ComputedStyle,
        on_click: Option<ClickHandlers>,
        on_hover: Option<UiAction>,
        tooltip: Option<Box<Self>>,
        popup: Option<StyledPopup>,
        panel: Option<StyledPanel>,
    },
    Progress {
        path: NodePath,
        node_key: Option<NodeKey>,
        value: ProgressValue,
        orientation: Orientation,
        style: ComputedStyle,
        on_click: Option<ClickHandlers>,
        on_hover: Option<UiAction>,
        tooltip: Option<Box<Self>>,
        popup: Option<StyledPopup>,
        panel: Option<StyledPanel>,
    },
    Rect {
        path: NodePath,
        node_key: Option<NodeKey>,
        style: ComputedStyle,
        on_click: Option<ClickHandlers>,
        on_hover: Option<UiAction>,
        tooltip: Option<Box<Self>>,
        popup: Option<StyledPopup>,
        panel: Option<StyledPanel>,
    },
    Image {
        path: NodePath,
        node_key: Option<NodeKey>,
        data: BinaryData,
        pixel_size: Size,
        style: ComputedStyle,
        tooltip: Option<Box<Self>>,
        popup: Option<StyledPopup>,
        panel: Option<StyledPanel>,
    },
    Module {
        path: NodePath,
        node_key: Option<NodeKey>,
        key: ModuleKey,
        options: ModuleOptions,
        style: ComputedStyle,
        on_click: Option<ClickHandlers>,
        on_hover: Option<UiAction>,
        tooltip: Option<Box<Self>>,
        popup: Option<StyledPopup>,
        panel: Option<StyledPanel>,
    },
}

impl StyledNode {
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
    pub const fn style(&self) -> &ComputedStyle {
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
            Self::Flex { on_hover, .. }
            | Self::Grid { on_hover, .. }
            | Self::Text { on_hover, .. }
            | Self::Progress { on_hover, .. }
            | Self::Rect { on_hover, .. }
            | Self::Module { on_hover, .. } => on_hover.as_ref(),
            Self::Image { .. } => None,
        }
    }

    #[must_use]
    pub fn tooltip(&self) -> Option<&Self> {
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
