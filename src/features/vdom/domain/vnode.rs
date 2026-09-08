use super::action::{ClickHandlers, UiAction};
use super::identifier::{NodeId, NodeKey};
use super::kind::VNodeKind;
use super::tag::NodeTag;
use crate::features::styling::domain::{ClassNameList, ElementId};
use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct VNode {
    #[serde(skip_deserializing, default = "NodeId::new")]
    pub(crate) node_id: NodeId,
    #[serde(default)]
    pub(crate) key: Option<NodeKey>,
    #[serde(default)]
    pub(crate) id: Option<ElementId>,
    #[serde(default)]
    pub(crate) class: Option<ClassNameList>,
    #[serde(default)]
    pub(crate) on_click: Option<ClickHandlers>,
    #[serde(default)]
    pub(crate) on_hover: Option<UiAction>,
    #[serde(default)]
    pub(crate) tooltip: Option<Box<Self>>,
    #[serde(default)]
    pub(crate) popup: Option<super::floating::PopupSpec>,
    #[serde(default)]
    pub(crate) panel: Option<super::floating::PanelSpec>,
    #[serde(flatten)]
    pub(crate) kind: VNodeKind,
}

impl VNode {
    #[must_use]
    pub const fn node_id(&self) -> NodeId {
        self.node_id
    }

    #[must_use]
    pub const fn with_node_id(mut self, node_id: NodeId) -> Self {
        self.node_id = node_id;
        self
    }

    #[must_use]
    pub const fn key(&self) -> Option<&NodeKey> {
        self.key.as_ref()
    }

    #[must_use]
    pub fn with_key(mut self, key: NodeKey) -> Self {
        self.key = Some(key);
        self
    }

    #[must_use]
    pub const fn element_id(&self) -> Option<&ElementId> {
        self.id.as_ref()
    }

    #[must_use]
    pub const fn class_names(&self) -> Option<&ClassNameList> {
        self.class.as_ref()
    }

    #[must_use]
    pub const fn on_click(&self) -> Option<&ClickHandlers> {
        self.on_click.as_ref()
    }

    #[must_use]
    pub const fn on_hover(&self) -> Option<&UiAction> {
        self.on_hover.as_ref()
    }

    #[must_use]
    pub fn tooltip(&self) -> Option<&Self> {
        self.tooltip.as_deref()
    }

    #[must_use]
    pub const fn popup(&self) -> Option<&super::floating::PopupSpec> {
        self.popup.as_ref()
    }

    #[must_use]
    pub fn with_popup(mut self, popup: super::floating::PopupSpec) -> Self {
        self.popup = Some(popup);
        self
    }

    #[must_use]
    pub const fn panel(&self) -> Option<&super::floating::PanelSpec> {
        self.panel.as_ref()
    }

    #[must_use]
    pub fn with_panel(mut self, panel: super::floating::PanelSpec) -> Self {
        self.panel = Some(panel);
        self
    }

    #[must_use]
    pub const fn tag(&self) -> NodeTag {
        match &self.kind {
            VNodeKind::Flex { .. } => NodeTag::Flex,
            VNodeKind::Grid { .. } => NodeTag::Grid,
            VNodeKind::Text { .. } => NodeTag::Text,
            VNodeKind::Progress { .. } => NodeTag::Progress,
            VNodeKind::Rect => NodeTag::Rect,
            VNodeKind::Image { .. } => NodeTag::Image,
            VNodeKind::Module { .. } => NodeTag::Module,
        }
    }

    #[must_use]
    pub const fn kind(&self) -> &VNodeKind {
        &self.kind
    }

    pub const fn kind_mut(&mut self) -> &mut VNodeKind {
        &mut self.kind
    }

    #[must_use]
    pub const fn children(&self) -> &[Self] {
        match &self.kind {
            VNodeKind::Flex { children } | VNodeKind::Grid { children } => children.as_slice(),
            _ => &[],
        }
    }

    pub const fn children_mut(&mut self) -> Option<&mut Vec<Self>> {
        match &mut self.kind {
            VNodeKind::Flex { children } | VNodeKind::Grid { children } => Some(children),
            _ => None,
        }
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        match &self.kind {
            VNodeKind::Flex { children } | VNodeKind::Grid { children } => children.is_empty(),
            VNodeKind::Text { text } => text.as_str().trim().is_empty(),
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::tag::TextContent;
    use crate::shared::primitives::geometry::Size;
    use crate::shared::primitives::{ModuleInstanceId, ModuleName, ModuleOptions};
    use std::collections::HashMap;

    #[test]
    fn test_vnode_constructors_and_accessors() {
        let text_node = VNode::new_text(
            TextContent::new("clock".to_string()),
            None,
            None,
            None,
            None,
            None,
        );
        assert_eq!(text_node.tag(), NodeTag::Text);
        assert!(text_node.tag().is_leaf());
        assert_eq!(text_node.children().len(), 0);

        let flex_node = VNode::new_flex(vec![text_node], None, None, None, None, None);
        assert_eq!(flex_node.tag(), NodeTag::Flex);
        assert!(flex_node.tag().is_container());
        assert_eq!(flex_node.children().len(), 1);
        assert_eq!(flex_node.children()[0].tag(), NodeTag::Text);
    }

    #[test]
    fn test_vnode_serde_deserialization() {
        let json = r#"{
            "type": "flex",
            "key": "main_bar",
            "children": [
                {
                    "type": "text",
                    "text": "12:00"
                }
            ]
        }"#;

        let node: VNode = serde_json::from_str(json).expect("Deserialization failed");
        assert_eq!(node.tag(), NodeTag::Flex);
        assert_eq!(node.key().unwrap().as_str(), "main_bar");
        assert_eq!(node.children().len(), 1);
        assert_eq!(node.children()[0].tag(), NodeTag::Text);
    }

    #[test]
    fn test_vnode_ignores_deserialized_node_id() {
        let fake_uuid = "00000000-0000-0000-0000-000000000000";
        let json = format!(
            r#"{{
            "type": "text",
            "text": "test",
            "node_id": "{fake_uuid}"
        }}"#
        );

        let node: VNode = serde_json::from_str(&json).expect("Deserialization failed");
        assert_ne!(node.node_id().to_string(), fake_uuid);
    }

    #[test]
    fn test_vnode_module_serde_and_constructor() {
        let mut opts_map = HashMap::new();
        opts_map.insert(
            "format".to_string(),
            crate::shared::primitives::DynamicValue::from("%H:%M"),
        );
        let opts = ModuleOptions::new(opts_map);
        let module_node = VNode::new_module(
            crate::features::vdom::domain::ModuleParams::new(
                ModuleName::new("clock"),
                Some(ModuleInstanceId::new("h1")),
                opts,
            ),
            None,
            None,
            None,
            None,
            None,
        );
        assert_eq!(module_node.tag(), NodeTag::Module);

        let json = r#"{
            "type": "module",
            "name": "clock",
            "instance_id": "h1",
            "options": {
                "format": "%H:%M:%S"
            }
        }"#;
        let deserialized: VNode = serde_json::from_str(json).expect("Deserialization failed");
        assert_eq!(deserialized.tag(), NodeTag::Module);
        if let VNodeKind::Module {
            name,
            instance_id,
            options,
        } = deserialized.kind()
        {
            assert_eq!(name.as_str(), "clock");
            assert_eq!(
                instance_id
                    .as_ref()
                    .map(crate::shared::primitives::ModuleInstanceId::as_str),
                Some("h1")
            );
            assert_eq!(
                options.get("format").and_then(|v| v.as_str()),
                Some("%H:%M:%S")
            );
        } else {
            panic!("Expected VNodeKind::Module");
        }
    }

    #[test]
    fn test_vnode_image_debug_omission() {
        let node = VNode::new_image(
            vec![1, 2, 3, 4, 5, 6, 7, 8],
            Size::new(2, 1),
            None,
            None,
            None,
        );
        let debug_str = format!("{node:?}");
        assert!(debug_str.contains("<Binary Data (8 bytes)>"));
        assert!(!debug_str.contains("1, 2, 3, 4"));
    }

    #[test]
    fn test_vnode_is_empty() {
        let empty_flex = VNode::new_flex(vec![], None, None, None, None, None);
        assert!(empty_flex.is_empty());

        let empty_text = VNode::new_text(
            TextContent::new("   ".to_string()),
            None,
            None,
            None,
            None,
            None,
        );
        assert!(empty_text.is_empty());

        let non_empty_text = VNode::new_text(
            TextContent::new("hello".to_string()),
            None,
            None,
            None,
            None,
            None,
        );
        assert!(!non_empty_text.is_empty());

        let non_empty_flex = VNode::new_flex(vec![non_empty_text], None, None, None, None, None);
        assert!(!non_empty_flex.is_empty());
    }

    #[test]
    fn test_grid_json_deserialization() {
        let json = r#"{
            "type": "grid",
            "children": [
                { "type": "text", "text": "Cell 1" }
            ]
        }"#;
        let node: VNode = serde_json::from_str(json).unwrap();
        assert_eq!(node.tag(), NodeTag::Grid);
        assert_eq!(node.children().len(), 1);
    }
}
