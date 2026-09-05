use super::action::{ClickHandlers, UiAction};
use super::identifier::NodeId;
use super::kind::VNodeKind;
use super::tag::TextContent;
use super::vnode::VNode;
use crate::features::styling::domain::{ClassNameList, ElementId, Orientation, ProgressValue};
use crate::shared::primitives::geometry::Size;
use crate::shared::primitives::{
    BinaryData, ModuleInstanceId, ModuleName, ModuleOptions,
};

impl VNode {
    #[must_use]
    pub fn new_flex(
        children: Vec<Self>,
        class: Option<ClassNameList>,
        id: Option<ElementId>,
        on_click: Option<ClickHandlers>,
        on_hover: Option<UiAction>,
        tooltip: Option<Box<Self>>,
    ) -> Self {
        Self {
            node_id: NodeId::new(),
            key: None,
            id,
            class,
            on_click,
            on_hover,
            tooltip,
            popup: None,
            kind: VNodeKind::Flex { children },
        }
    }

    #[must_use]
    pub fn new_grid(
        children: Vec<Self>,
        class: Option<ClassNameList>,
        id: Option<ElementId>,
        on_click: Option<ClickHandlers>,
        on_hover: Option<UiAction>,
        tooltip: Option<Box<Self>>,
    ) -> Self {
        Self {
            node_id: NodeId::new(),
            key: None,
            id,
            class,
            on_click,
            on_hover,
            tooltip,
            popup: None,
            kind: VNodeKind::Grid { children },
        }
    }

    #[must_use]
    pub fn new_text(
        text: TextContent,
        class: Option<ClassNameList>,
        id: Option<ElementId>,
        on_click: Option<ClickHandlers>,
        on_hover: Option<UiAction>,
        tooltip: Option<Box<Self>>,
    ) -> Self {
        Self {
            node_id: NodeId::new(),
            key: None,
            id,
            class,
            on_click,
            on_hover,
            tooltip,
            popup: None,
            kind: VNodeKind::Text { text },
        }
    }

    #[must_use]
    pub fn new_progress(
        value: ProgressValue,
        orientation: Orientation,
        class: Option<ClassNameList>,
        id: Option<ElementId>,
        on_click: Option<ClickHandlers>,
        on_hover: Option<UiAction>,
        tooltip: Option<Box<Self>>,
    ) -> Self {
        Self {
            node_id: NodeId::new(),
            key: None,
            id,
            class,
            on_click,
            on_hover,
            tooltip,
            popup: None,
            kind: VNodeKind::Progress { value, orientation },
        }
    }

    #[must_use]
    pub fn new_rect(
        class: Option<ClassNameList>,
        id: Option<ElementId>,
        on_click: Option<ClickHandlers>,
        on_hover: Option<UiAction>,
        tooltip: Option<Box<Self>>,
    ) -> Self {
        Self {
            node_id: NodeId::new(),
            key: None,
            id,
            class,
            on_click,
            on_hover,
            tooltip,
            popup: None,
            kind: VNodeKind::Rect,
        }
    }

    #[must_use]
    pub fn new_image(
        data: impl Into<BinaryData>,
        pixel_size: Size,
        class: Option<ClassNameList>,
        id: Option<ElementId>,
        tooltip: Option<Box<Self>>,
    ) -> Self {
        Self {
            node_id: NodeId::new(),
            key: None,
            id,
            class,
            on_click: None,
            on_hover: None,
            tooltip,
            popup: None,
            kind: VNodeKind::Image {
                data: data.into(),
                pixel_size,
            },
        }
    }

    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub fn new_module(
        name: ModuleName,
        instance_id: Option<ModuleInstanceId>,
        options: ModuleOptions,
        class: Option<ClassNameList>,
        id: Option<ElementId>,
        on_click: Option<ClickHandlers>,
        on_hover: Option<UiAction>,
        tooltip: Option<Box<Self>>,
    ) -> Self {
        Self {
            node_id: NodeId::new(),
            key: None,
            id,
            class,
            on_click,
            on_hover,
            tooltip,
            popup: None,
            kind: VNodeKind::Module {
                name,
                instance_id,
                options,
            },
        }
    }
}
