use crate::features::styling::domain::{ComputedStyle, Orientation, ProgressValue};
use crate::features::vdom::domain::{ClickHandlers, NodePath, TextContent, UiAction};
use crate::shared::primitives::geometry::Size;
use crate::shared::primitives::{BinaryData, ModuleKey, ModuleOptions};

#[derive(Debug, Clone, PartialEq)]
pub enum StyledNode {
    Flex {
        path: NodePath,
        children: Vec<Self>,
        style: ComputedStyle,
        on_click: Option<ClickHandlers>,
        on_hover: Option<UiAction>,
        tooltip: Option<Box<Self>>,
        popup: Option<Box<Self>>,
    },
    Grid {
        path: NodePath,
        children: Vec<Self>,
        style: ComputedStyle,
        on_click: Option<ClickHandlers>,
        on_hover: Option<UiAction>,
        tooltip: Option<Box<Self>>,
        popup: Option<Box<Self>>,
    },
    Text {
        path: NodePath,
        text: TextContent,
        style: ComputedStyle,
        on_click: Option<ClickHandlers>,
        on_hover: Option<UiAction>,
        tooltip: Option<Box<Self>>,
        popup: Option<Box<Self>>,
    },
    Progress {
        path: NodePath,
        value: ProgressValue,
        orientation: Orientation,
        style: ComputedStyle,
        on_click: Option<ClickHandlers>,
        on_hover: Option<UiAction>,
        tooltip: Option<Box<Self>>,
        popup: Option<Box<Self>>,
    },
    Rect {
        path: NodePath,
        style: ComputedStyle,
        on_click: Option<ClickHandlers>,
        on_hover: Option<UiAction>,
        tooltip: Option<Box<Self>>,
        popup: Option<Box<Self>>,
    },
    Image {
        path: NodePath,
        data: BinaryData,
        pixel_size: Size,
        style: ComputedStyle,
        tooltip: Option<Box<Self>>,
        popup: Option<Box<Self>>,
    },
    Module {
        path: NodePath,
        key: ModuleKey,
        options: ModuleOptions,
        style: ComputedStyle,
        on_click: Option<ClickHandlers>,
        on_hover: Option<UiAction>,
        tooltip: Option<Box<Self>>,
        popup: Option<Box<Self>>,
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
    pub fn popup(&self) -> Option<&Self> {
        match self {
            Self::Flex { popup, .. }
            | Self::Grid { popup, .. }
            | Self::Text { popup, .. }
            | Self::Progress { popup, .. }
            | Self::Rect { popup, .. }
            | Self::Image { popup, .. }
            | Self::Module { popup, .. } => popup.as_deref(),
        }
    }
}
