//! Constructors that build a `VNode` of each kind with a fresh `NodeId`, no key and no popup or
//! panel.
use super::action::{ClickHandlers, UiAction};
use super::identifier::NodeId;
use super::kind::{ModuleParams, VNodeKind};
use super::tag::TextContent;
use super::vnode::VNode;
use crate::features::styling::domain::{ClassNameList, ElementId, Orientation, ProgressValue};
use crate::shared::primitives::BinaryData;
use crate::shared::primitives::geometry::Size;

impl VNode {
    /// Builds a flex container node, with a fresh `NodeId`, no key and no popup or panel.
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
            panel: None,
            kind: VNodeKind::Flex { children },
        }
    }

    #[must_use]
    pub(crate) fn new_grid(
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
            panel: None,
            kind: VNodeKind::Grid { children },
        }
    }

    /// Builds a text node, with a fresh `NodeId`, no key and no popup or panel.
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
            panel: None,
            kind: VNodeKind::Text { text },
        }
    }

    #[must_use]
    pub(crate) fn new_progress(
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
            panel: None,
            kind: VNodeKind::Progress { value, orientation },
        }
    }

    #[must_use]
    pub(crate) fn new_rect(
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
            panel: None,
            kind: VNodeKind::Rect,
        }
    }

    #[must_use]
    pub(crate) fn new_image(
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
            panel: None,
            kind: VNodeKind::Image {
                data: data.into(),
                pixel_size,
            },
        }
    }

    #[must_use]
    pub(crate) fn new_module(
        params: ModuleParams,
        class: Option<ClassNameList>,
        id: Option<ElementId>,
        on_click: Option<ClickHandlers>,
        on_hover: Option<UiAction>,
        tooltip: Option<Box<Self>>,
    ) -> Self {
        let (name, instance_id, options) = params.into_parts();
        Self {
            node_id: NodeId::new(),
            key: None,
            id,
            class,
            on_click,
            on_hover,
            tooltip,
            popup: None,
            panel: None,
            kind: VNodeKind::Module {
                name,
                instance_id,
                options,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::events::core::PointerButton;
    use crate::shared::primitives::{DynamicValue, ModuleInstanceId, ModuleName, ModuleOptions};
    use std::collections::HashMap;

    fn class(label: &str) -> ClassNameList {
        ClassNameList::parse(&format!("{label}-class")).unwrap()
    }

    fn element_id(label: &str) -> ElementId {
        ElementId::new(format!("{label}-id")).unwrap()
    }

    fn on_click(label: &str) -> ClickHandlers {
        let mut handlers = ClickHandlers::new();
        handlers.insert(
            PointerButton::Right,
            UiAction::Exec(format!("{label}-click")),
        );
        handlers
    }

    fn on_hover(label: &str) -> UiAction {
        UiAction::Exec(format!("{label}-hover"))
    }

    fn tooltip(label: &str) -> Box<VNode> {
        Box::new(VNode::new_text(
            TextContent::new(format!("{label}-tip")),
            None,
            None,
            None,
            None,
            None,
        ))
    }

    /// Asserts `node` carries the class, id and tooltip made from `label`, and its click and
    /// hover handlers when `handlers` names it.
    fn assert_attributes(node: &VNode, label: &str, handlers: Option<&str>) {
        assert_eq!(node.class_names(), Some(&class(label)));
        assert_eq!(node.element_id(), Some(&element_id(label)));
        assert_eq!(node.on_click(), handlers.map(on_click).as_ref());
        assert_eq!(node.on_hover(), handlers.map(on_hover).as_ref());
        assert_eq!(node.tooltip().map(VNode::kind), Some(tooltip(label).kind()));
        assert_eq!(node.key(), None);
        assert_eq!(node.popup(), None);
        assert_eq!(node.panel(), None);
    }

    #[test]
    fn test_container_constructors_keep_each_attribute_in_place() {
        let child = VNode::new_rect(None, None, None, None, None);
        let flex = VNode::new_flex(
            vec![child.clone()],
            Some(class("flex")),
            Some(element_id("flex")),
            Some(on_click("flex")),
            Some(on_hover("flex")),
            Some(tooltip("flex")),
        );
        assert_attributes(&flex, "flex", Some("flex"));
        assert_eq!(
            flex.kind(),
            &VNodeKind::Flex {
                children: vec![child.clone()]
            }
        );

        let grid = VNode::new_grid(
            vec![child.clone(), child],
            Some(class("grid")),
            Some(element_id("grid")),
            Some(on_click("grid")),
            Some(on_hover("grid")),
            Some(tooltip("grid")),
        );
        assert_attributes(&grid, "grid", Some("grid"));
        assert_eq!(grid.children().len(), 2);
        assert!(matches!(grid.kind(), VNodeKind::Grid { .. }));
    }

    #[test]
    fn test_leaf_constructors_keep_each_attribute_in_place() {
        let text = VNode::new_text(
            TextContent::new("hello".to_string()),
            Some(class("text")),
            Some(element_id("text")),
            Some(on_click("text")),
            Some(on_hover("text")),
            Some(tooltip("text")),
        );
        assert_attributes(&text, "text", Some("text"));
        assert_eq!(
            text.kind(),
            &VNodeKind::Text {
                text: TextContent::new("hello".to_string())
            }
        );

        let progress = VNode::new_progress(
            ProgressValue::new(0.3).unwrap(),
            Orientation::Vertical,
            Some(class("bar")),
            Some(element_id("bar")),
            Some(on_click("bar")),
            Some(on_hover("bar")),
            Some(tooltip("bar")),
        );
        assert_attributes(&progress, "bar", Some("bar"));
        assert_eq!(
            progress.kind(),
            &VNodeKind::Progress {
                value: ProgressValue::new(0.3).unwrap(),
                orientation: Orientation::Vertical,
            }
        );

        let rect = VNode::new_rect(
            Some(class("rect")),
            Some(element_id("rect")),
            Some(on_click("rect")),
            Some(on_hover("rect")),
            Some(tooltip("rect")),
        );
        assert_attributes(&rect, "rect", Some("rect"));
        assert_eq!(rect.kind(), &VNodeKind::Rect);
    }

    #[test]
    fn test_image_and_module_constructors_keep_each_attribute_in_place() {
        let image = VNode::new_image(
            vec![1, 2, 3, 4],
            Size::new(1, 1),
            Some(class("icon")),
            Some(element_id("icon")),
            Some(tooltip("icon")),
        );
        assert_attributes(&image, "icon", None);
        assert_eq!(
            image.kind(),
            &VNodeKind::Image {
                data: BinaryData::new(vec![1, 2, 3, 4]),
                pixel_size: Size::new(1, 1),
            }
        );

        let options = ModuleOptions::new(HashMap::from([(
            "format".to_string(),
            DynamicValue::String("%H".to_string()),
        )]));
        let module = VNode::new_module(
            ModuleParams::new(
                ModuleName::new("clock"),
                Some(ModuleInstanceId::new("left")),
                options.clone(),
            ),
            Some(class("clock")),
            Some(element_id("clock")),
            Some(on_click("clock")),
            Some(on_hover("clock")),
            Some(tooltip("clock")),
        );
        assert_attributes(&module, "clock", Some("clock"));
        assert_eq!(
            module.kind(),
            &VNodeKind::Module {
                name: ModuleName::new("clock"),
                instance_id: Some(ModuleInstanceId::new("left")),
                options,
            }
        );
    }
}
