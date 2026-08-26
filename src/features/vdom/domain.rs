use crate::app::commands::AppCommand;
use crate::features::layout_engine::domain::StyledNode;
use crate::features::styling::domain::{
    ClassNameList, ElementId, ElementQuery, Orientation, ProgressValue,
};
use crate::features::styling::ports::StyleResolverPort;
use crate::shared::primitives::geometry::Size;
use crate::shared::primitives::{BinaryData, ModuleInstanceId, ModuleKey, ModuleName, ModuleOptions};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum VdomError {
    #[error("Invalid NodeKey: {0}")]
    InvalidNodeKey(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NodeId(Uuid);

impl NodeId {
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    #[must_use]
    pub const fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    #[must_use]
    pub const fn uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Default for NodeId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct NodeKey(String);

impl NodeKey {
    /// Creates a new `NodeKey`.
    ///
    /// # Errors
    ///
    /// Returns `VdomError::InvalidNodeKey` if the key is empty or whitespace.
    pub fn new(key: impl Into<String>) -> Result<Self, VdomError> {
        let s = key.into();
        if s.trim().is_empty() {
            return Err(VdomError::InvalidNodeKey(
                "NodeKey cannot be empty or whitespace".to_string(),
            ));
        }
        Ok(Self(s))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for NodeKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<'de> Deserialize<'de> for NodeKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Self::new(s).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeTag {
    Flex,
    Text,
    Progress,
    Rect,
    Image,
    Module,
}

impl NodeTag {
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Flex => "flex",
            Self::Text => "text",
            Self::Progress => "progress",
            Self::Rect => "rect",
            Self::Image => "image",
            Self::Module => "module",
        }
    }

    #[must_use]
    pub const fn is_container(&self) -> bool {
        matches!(self, Self::Flex)
    }

    #[must_use]
    pub const fn is_leaf(&self) -> bool {
        !self.is_container()
    }
}

impl std::fmt::Display for NodeTag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct TextContent {
    text: String,
}

impl TextContent {
    #[must_use]
    pub const fn new(text: String) -> Self {
        Self { text }
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.text
    }
}

impl std::str::FromStr for TextContent {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::new(s.to_string()))
    }
}

impl From<&str> for TextContent {
    fn from(s: &str) -> Self {
        Self::new(s.to_string())
    }
}

impl From<String> for TextContent {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

impl std::fmt::Display for TextContent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.text)
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "type")]
pub enum VNodeKind {
    #[serde(rename = "flex")]
    Flex {
        #[serde(default)]
        children: Vec<VNode>,
    },
    #[serde(rename = "text")]
    Text { text: TextContent },
    #[serde(rename = "progress")]
    Progress {
        #[serde(default)]
        value: ProgressValue,
        #[serde(default)]
        orientation: Orientation,
    },
    #[serde(rename = "rect")]
    Rect,
    #[serde(rename = "image")]
    Image {
        data: BinaryData,
        pixel_size: Size,
    },
    #[serde(rename = "module")]
    Module {
        name: ModuleName,
        #[serde(default)]
        instance_id: Option<ModuleInstanceId>,
        #[serde(default)]
        options: ModuleOptions,
    },
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct VNode {
    #[serde(skip_deserializing, default = "NodeId::new")]
    node_id: NodeId,
    #[serde(default)]
    key: Option<NodeKey>,
    #[serde(default)]
    id: Option<ElementId>,
    #[serde(default)]
    class: Option<ClassNameList>,
    #[serde(default)]
    on_click: Option<AppCommand>,
    #[serde(default)]
    on_hover: Option<AppCommand>,
    #[serde(default)]
    tooltip: Option<Box<Self>>,
    #[serde(flatten)]
    kind: VNodeKind,
}

impl VNode {
    #[must_use]
    pub fn new_flex(
        children: Vec<Self>,
        class: Option<ClassNameList>,
        id: Option<ElementId>,
        on_click: Option<AppCommand>,
        on_hover: Option<AppCommand>,
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
            kind: VNodeKind::Flex { children },
        }
    }

    #[must_use]
    pub fn new_text(
        text: TextContent,
        class: Option<ClassNameList>,
        id: Option<ElementId>,
        on_click: Option<AppCommand>,
        on_hover: Option<AppCommand>,
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
            kind: VNodeKind::Text { text },
        }
    }

    #[must_use]
    pub fn new_progress(
        value: ProgressValue,
        orientation: Orientation,
        class: Option<ClassNameList>,
        id: Option<ElementId>,
        on_click: Option<AppCommand>,
        on_hover: Option<AppCommand>,
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
            kind: VNodeKind::Progress { value, orientation },
        }
    }

    #[must_use]
    pub fn new_rect(
        class: Option<ClassNameList>,
        id: Option<ElementId>,
        on_click: Option<AppCommand>,
        on_hover: Option<AppCommand>,
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
        on_click: Option<AppCommand>,
        on_hover: Option<AppCommand>,
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
            kind: VNodeKind::Module {
                name,
                instance_id,
                options,
            },
        }
    }

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
    pub const fn on_click(&self) -> Option<&AppCommand> {
        self.on_click.as_ref()
    }

    #[must_use]
    pub const fn on_hover(&self) -> Option<&AppCommand> {
        self.on_hover.as_ref()
    }

    #[must_use]
    pub fn tooltip(&self) -> Option<&Self> {
        self.tooltip.as_deref()
    }

    #[must_use]
    pub const fn tag(&self) -> NodeTag {
        match &self.kind {
            VNodeKind::Flex { .. } => NodeTag::Flex,
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
            VNodeKind::Flex { children } => children.as_slice(),
            _ => &[],
        }
    }

    pub const fn children_mut(&mut self) -> Option<&mut Vec<Self>> {
        match &mut self.kind {
            VNodeKind::Flex { children } => Some(children),
            _ => None,
        }
    }

    #[must_use]
    pub fn resolve_styles(
        &self,
        resolver: &dyn StyleResolverPort,
        parent: Option<&ElementQuery>,
    ) -> StyledNode {
        let classes_slice = self
            .class
            .as_ref()
            .map_or(&[][..], ClassNameList::as_slice);
        let query = ElementQuery::new(
            self.tag().as_str(),
            self.id.as_ref(),
            classes_slice,
            &[],
            parent,
        );
        let style = resolver.resolve_style(&query);
        let styled_tooltip = self
            .tooltip
            .as_ref()
            .map(|t| Box::new(t.resolve_styles(resolver, None)));

        match &self.kind {
            VNodeKind::Flex { children } => {
                let styled_children = children
                    .iter()
                    .map(|child| child.resolve_styles(resolver, Some(&query)))
                    .collect();
                StyledNode::Flex {
                    children: styled_children,
                    style,
                    on_click: self.on_click.clone(),
                    on_hover: self.on_hover.clone(),
                    tooltip: styled_tooltip,
                }
            }
            VNodeKind::Text { text } => StyledNode::Text {
                text: text.clone(),
                style,
                on_click: self.on_click.clone(),
                on_hover: self.on_hover.clone(),
                tooltip: styled_tooltip,
            },
            VNodeKind::Progress { value, orientation } => StyledNode::Progress {
                value: *value,
                orientation: *orientation,
                style,
                on_click: self.on_click.clone(),
                on_hover: self.on_hover.clone(),
                tooltip: styled_tooltip,
            },
            VNodeKind::Rect => StyledNode::Rect {
                style,
                on_click: self.on_click.clone(),
                on_hover: self.on_hover.clone(),
                tooltip: styled_tooltip,
            },
            VNodeKind::Image { data, pixel_size } => StyledNode::Image {
                data: data.clone(),
                pixel_size: *pixel_size,
                style,
                tooltip: styled_tooltip,
            },
            VNodeKind::Module {
                name,
                instance_id,
                options,
            } => StyledNode::Module {
                key: ModuleKey::new(name.clone(), instance_id.clone()),
                options: options.clone(),
                style,
                on_click: self.on_click.clone(),
                on_hover: self.on_hover.clone(),
                tooltip: styled_tooltip,
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Patch {
    NoChange,
    Replace {
        old_node_id: NodeId,
        new_node: Box<VNode>,
    },
    UpdateProps {
        node_id: NodeId,
        class_changed: bool,
        id_changed: bool,
        handlers_changed: bool,
        tooltip_patch: Option<Box<Self>>,
        kind_patch: Box<Self>,
    },
    UpdateText {
        node_id: NodeId,
        new_text: TextContent,
    },
    UpdateProgress {
        node_id: NodeId,
        new_value: ProgressValue,
        new_orientation: Orientation,
    },
    UpdateImage {
        node_id: NodeId,
        new_data: BinaryData,
        new_pixel_size: Size,
    },
    UpdateModule {
        node_id: NodeId,
        new_name: ModuleName,
        new_instance_id: Option<ModuleInstanceId>,
        new_options: ModuleOptions,
    },
    UpdateChildren {
        node_id: NodeId,
        child_patches: Vec<ChildPatchOp>,
    },
}

impl Patch {
    #[must_use]
    pub const fn is_no_change(&self) -> bool {
        matches!(self, Self::NoChange)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ChildPatchOp {
    Insert {
        index: usize,
        node: Box<VNode>,
    },
    Remove {
        node_id: NodeId,
        index: usize,
    },
    Move {
        node_id: NodeId,
        from: usize,
        to: usize,
    },
    Update {
        node_id: NodeId,
        patch: Box<Patch>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct DiffResult {
    patch: Patch,
}

impl DiffResult {
    #[must_use]
    pub const fn new(patch: Patch) -> Self {
        Self { patch }
    }

    #[must_use]
    pub const fn unchanged() -> Self {
        Self {
            patch: Patch::NoChange,
        }
    }

    #[must_use]
    pub const fn is_unchanged(&self) -> bool {
        self.patch.is_no_change()
    }

    #[must_use]
    pub const fn patch(&self) -> &Patch {
        &self.patch
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_node_id_creation_and_uniqueness() {
        let id1 = NodeId::new();
        let id2 = NodeId::new();
        assert_ne!(id1, id2);
        assert_eq!(id1, NodeId::from_uuid(*id1.uuid()));
        assert!(!id1.to_string().is_empty());
    }

    #[test]
    fn test_node_key_validation() {
        assert!(NodeKey::new("valid-key_123").is_ok());
        assert!(NodeKey::new("").is_err());
        assert!(NodeKey::new("   ").is_err());

        let key = NodeKey::new("tab-1").unwrap();
        assert_eq!(key.as_str(), "tab-1");
        assert_eq!(key.to_string(), "tab-1");
    }

    #[test]
    fn test_text_content() {
        let text = TextContent::new("Hello World".to_string());
        assert_eq!(text.as_str(), "Hello World");
        assert_eq!(text.to_string(), "Hello World");
    }

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
        opts_map.insert("format".to_string(), crate::shared::primitives::DynamicValue::from("%H:%M"));
        let opts = ModuleOptions::new(opts_map);
        let module_node = VNode::new_module(
            ModuleName::new("hour"),
            Some(ModuleInstanceId::new("h1")),
            opts,
            None,
            None,
            None,
            None,
            None,
        );
        assert_eq!(module_node.tag(), NodeTag::Module);

        let json = r#"{
            "type": "module",
            "name": "hour",
            "instance_id": "h1",
            "options": {
                "format": "%H:%M:%S"
            }
        }"#;
        let deserialized: VNode = serde_json::from_str(json).expect("Deserialization failed");
        assert_eq!(deserialized.tag(), NodeTag::Module);
        if let VNodeKind::Module { name, instance_id, options } = deserialized.kind() {
            assert_eq!(name.as_str(), "hour");
            assert_eq!(instance_id.as_ref().map(crate::shared::primitives::ModuleInstanceId::as_str), Some("h1"));
            assert_eq!(options.get("format").and_then(|v| v.as_str()), Some("%H:%M:%S"));
        } else {
            panic!("Expected VNodeKind::Module");
        }
    }

    #[test]
    fn test_diff_result() {
        let res = DiffResult::unchanged();
        assert!(res.is_unchanged());
        assert_eq!(res.patch(), &Patch::NoChange);
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
}
