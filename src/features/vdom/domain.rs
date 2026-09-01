use crate::features::layout_engine::domain::StyledNode;
use crate::features::styling::domain::{
    ClassNameList, ComputedStyle, ElementId, ElementQuery, Orientation, ProgressValue, PseudoClass,
};
use crate::features::styling::ports::StyleResolverPort;
use crate::shared::events::core::PointerButton;
use crate::shared::primitives::geometry::{Position, Size};
use crate::shared::primitives::{
    BinaryData, ModuleInstanceId, ModuleKey, ModuleName, ModuleOptions,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub enum UiAction {
    Exec(String),
    SystrayAction {
        id: crate::features::systray::domain::SystrayId,
        action: crate::features::systray::domain::SystrayActionName,
        #[serde(default)]
        pos: Option<Position>,
    },
    ScriptCall(crate::shared::primitives::FunctionName),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UiCommand {
    Exec(String),
    SystrayAction {
        id: crate::features::systray::domain::SystrayId,
        action: crate::features::systray::domain::SystrayActionName,
        pos: Option<Position>,
    },
}

pub trait UiCommandSender: Send + Sync {
    fn send_ui_command(&self, cmd: UiCommand);
}

impl<F> UiCommandSender for F
where
    F: Fn(UiCommand) + Send + Sync,
{
    fn send_ui_command(&self, cmd: UiCommand) {
        self(cmd);
    }
}

impl UiCommandSender for tokio::sync::mpsc::Sender<UiCommand> {
    fn send_ui_command(&self, cmd: UiCommand) {
        if let Err(e) = self.try_send(cmd) {
            tracing::error!(?e, "failed to send ui command via tokio channel");
        }
    }
}

impl UiCommandSender for std::sync::mpsc::Sender<UiCommand> {
    fn send_ui_command(&self, cmd: UiCommand) {
        if let Err(e) = self.send(cmd) {
            tracing::error!(?e, "failed to send ui command via std channel");
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ClickHandlers {
    handlers: HashMap<PointerButton, UiAction>,
}

impl ClickHandlers {
    #[must_use]
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    #[must_use]
    pub fn from_single(action: UiAction) -> Self {
        let mut handlers = HashMap::new();
        handlers.insert(PointerButton::Left, action.clone());
        handlers.insert(PointerButton::Middle, action.clone());
        handlers.insert(PointerButton::Right, action);
        Self { handlers }
    }

    #[must_use]
    pub fn get(&self, button: &PointerButton) -> Option<&UiAction> {
        self.handlers.get(button)
    }

    pub fn insert(&mut self, button: PointerButton, action: UiAction) {
        self.handlers.insert(button, action);
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.handlers.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&PointerButton, &UiAction)> {
        self.handlers.iter()
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
enum ClickHandlersHelper {
    Single(UiAction),
    Map(HashMap<String, UiAction>),
}

impl<'de> Deserialize<'de> for ClickHandlers {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        match ClickHandlersHelper::deserialize(deserializer)? {
            ClickHandlersHelper::Single(action) => Ok(Self::from_single(action)),
            ClickHandlersHelper::Map(map) => {
                let mut handlers = HashMap::new();
                for (k, v) in map {
                    if let Some(btn) = PointerButton::from_name(&k) {
                        handlers.insert(btn, v);
                    }
                }
                Ok(Self { handlers })
            }
        }
    }
}

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

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct NodePath(Vec<usize>);

impl NodePath {
    #[must_use]
    pub const fn root() -> Self {
        Self(Vec::new())
    }

    #[must_use]
    pub const fn new(path: Vec<usize>) -> Self {
        Self(path)
    }

    #[must_use]
    pub fn child(&self, index: usize) -> Self {
        let mut new_path = self.0.clone();
        new_path.push(index);
        Self(new_path)
    }

    #[must_use]
    pub fn as_slice(&self) -> &[usize] {
        &self.0
    }

    #[must_use]
    pub const fn is_root(&self) -> bool {
        self.0.is_empty()
    }

    #[must_use]
    pub fn starts_with(&self, prefix: &Self) -> bool {
        self.0.starts_with(&prefix.0)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InteractionContext {
    hovered_path: Option<NodePath>,
    active_path: Option<NodePath>,
    focused_path: Option<NodePath>,
    is_monitor_focused: bool,
}

impl InteractionContext {
    #[must_use]
    pub const fn new(
        hovered_path: Option<NodePath>,
        active_path: Option<NodePath>,
        focused_path: Option<NodePath>,
        is_monitor_focused: bool,
    ) -> Self {
        Self {
            hovered_path,
            active_path,
            focused_path,
            is_monitor_focused,
        }
    }

    #[must_use]
    pub const fn hovered_path(&self) -> Option<&NodePath> {
        self.hovered_path.as_ref()
    }

    #[must_use]
    pub const fn active_path(&self) -> Option<&NodePath> {
        self.active_path.as_ref()
    }

    #[must_use]
    pub const fn focused_path(&self) -> Option<&NodePath> {
        self.focused_path.as_ref()
    }

    #[must_use]
    pub const fn is_monitor_focused(&self) -> bool {
        self.is_monitor_focused
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
    Grid,
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
            Self::Grid => "grid",
            Self::Text => "text",
            Self::Progress => "progress",
            Self::Rect => "rect",
            Self::Image => "image",
            Self::Module => "module",
        }
    }

    #[must_use]
    pub const fn is_container(&self) -> bool {
        matches!(self, Self::Flex | Self::Grid)
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
    #[serde(rename = "grid")]
    Grid {
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
    Image { data: BinaryData, pixel_size: Size },
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
    on_click: Option<ClickHandlers>,
    #[serde(default)]
    on_hover: Option<UiAction>,
    #[serde(default)]
    tooltip: Option<Box<Self>>,
    #[serde(default)]
    popup: Option<Box<Self>>,
    #[serde(flatten)]
    kind: VNodeKind,
}

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
    pub fn popup(&self) -> Option<&Self> {
        self.popup.as_deref()
    }

    #[must_use]
    pub fn with_popup(mut self, popup: Box<Self>) -> Self {
        self.popup = Some(popup);
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

    #[must_use]
    pub fn resolve_styles(
        &self,
        resolver: &dyn StyleResolverPort,
        interaction: Option<&InteractionContext>,
        parent: Option<&ElementQuery>,
    ) -> StyledNode {
        self.resolve_styles_recursive(resolver, interaction, &NodePath::root(), 0, 1, parent)
    }

    fn resolve_styles_recursive(
        &self,
        resolver: &dyn StyleResolverPort,
        interaction: Option<&InteractionContext>,
        path: &NodePath,
        child_index: usize,
        total_children: usize,
        parent: Option<&ElementQuery>,
    ) -> StyledNode {
        let pseudo_classes = compute_pseudo_classes(path, interaction);
        let classes_slice = self.class.as_ref().map_or(&[][..], ClassNameList::as_slice);
        let query = ElementQuery::new(
            self.tag().as_str(),
            self.id.as_ref(),
            classes_slice,
            &pseudo_classes,
            parent,
        )
        .with_structural_context(child_index, total_children, self.is_empty());

        let mut style = match &self.kind {
            VNodeKind::Flex { .. } => ComputedStyle::default_for_flex(),
            VNodeKind::Grid { .. } => ComputedStyle::default_for_grid(),
            _ => ComputedStyle::default(),
        };
        style.merge_with(&resolver.resolve_style(&query));

        let styled_tooltip = self.tooltip.as_ref().map(|t| {
            Box::new(t.resolve_styles_recursive(resolver, interaction, &path.child(0), 0, 1, None))
        });
        let styled_popup = self.popup.as_ref().map(|p| {
            Box::new(p.resolve_styles_recursive(resolver, interaction, &path.child(0), 0, 1, None))
        });

        let styled_children = match &self.kind {
            VNodeKind::Flex { children } | VNodeKind::Grid { children } => {
                let total = children.len();
                children
                    .iter()
                    .enumerate()
                    .map(|(idx, child)| {
                        child.resolve_styles_recursive(
                            resolver,
                            interaction,
                            &path.child(idx),
                            idx,
                            total,
                            Some(&query),
                        )
                    })
                    .collect()
            }
            _ => Vec::new(),
        };

        self.map_to_styled_node(path, style, styled_tooltip, styled_popup, styled_children)
    }

    fn map_to_styled_node(
        &self,
        path: &NodePath,
        style: ComputedStyle,
        styled_tooltip: Option<Box<StyledNode>>,
        styled_popup: Option<Box<StyledNode>>,
        styled_children: Vec<StyledNode>,
    ) -> StyledNode {
        match &self.kind {
            VNodeKind::Flex { .. } => StyledNode::Flex {
                path: path.clone(),
                children: styled_children,
                style,
                on_click: self.on_click.clone(),
                on_hover: self.on_hover.clone(),
                tooltip: styled_tooltip,
                popup: styled_popup,
            },
            VNodeKind::Grid { .. } => StyledNode::Grid {
                path: path.clone(),
                children: styled_children,
                style,
                on_click: self.on_click.clone(),
                on_hover: self.on_hover.clone(),
                tooltip: styled_tooltip,
                popup: styled_popup,
            },
            VNodeKind::Text { text } => StyledNode::Text {
                path: path.clone(),
                text: text.clone(),
                style,
                on_click: self.on_click.clone(),
                on_hover: self.on_hover.clone(),
                tooltip: styled_tooltip,
                popup: styled_popup,
            },
            VNodeKind::Progress { value, orientation } => StyledNode::Progress {
                path: path.clone(),
                value: *value,
                orientation: *orientation,
                style,
                on_click: self.on_click.clone(),
                on_hover: self.on_hover.clone(),
                tooltip: styled_tooltip,
                popup: styled_popup,
            },
            VNodeKind::Rect => StyledNode::Rect {
                path: path.clone(),
                style,
                on_click: self.on_click.clone(),
                on_hover: self.on_hover.clone(),
                tooltip: styled_tooltip,
                popup: styled_popup,
            },
            VNodeKind::Image { data, pixel_size } => StyledNode::Image {
                path: path.clone(),
                data: data.clone(),
                pixel_size: *pixel_size,
                style,
                tooltip: styled_tooltip,
                popup: styled_popup,
            },
            VNodeKind::Module {
                name,
                instance_id,
                options,
            } => StyledNode::Module {
                path: path.clone(),
                key: ModuleKey::new(name.clone(), instance_id.clone()),
                options: options.clone(),
                style,
                on_click: self.on_click.clone(),
                on_hover: self.on_hover.clone(),
                tooltip: styled_tooltip,
                popup: styled_popup,
            },
        }
    }
}

fn compute_pseudo_classes(
    path: &NodePath,
    interaction: Option<&InteractionContext>,
) -> Vec<PseudoClass> {
    let mut pseudo_classes = Vec::new();
    let Some(ctx) = interaction else {
        return pseudo_classes;
    };

    if ctx
        .hovered_path()
        .is_some_and(|h| h == path || h.starts_with(path))
    {
        pseudo_classes.push(PseudoClass::Hover);
    }
    if ctx
        .active_path()
        .is_some_and(|a| a == path || a.starts_with(path))
    {
        pseudo_classes.push(PseudoClass::Active);
    }
    if ctx.focused_path().is_some_and(|f| f == path) {
        pseudo_classes.push(PseudoClass::Focused);
    }
    if path.is_root() && ctx.is_monitor_focused() && !pseudo_classes.contains(&PseudoClass::Focused)
    {
        pseudo_classes.push(PseudoClass::Focused);
    }

    pseudo_classes
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
        popup_patch: Option<Box<Self>>,
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
        opts_map.insert(
            "format".to_string(),
            crate::shared::primitives::DynamicValue::from("%H:%M"),
        );
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
        if let VNodeKind::Module {
            name,
            instance_id,
            options,
        } = deserialized.kind()
        {
            assert_eq!(name.as_str(), "hour");
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

    #[test]
    fn test_node_path_operations() {
        let root = NodePath::root();
        assert!(root.is_root());
        let empty: &[usize] = &[];
        assert_eq!(root.as_slice(), empty);

        let child0 = root.child(0);
        assert!(!child0.is_root());
        assert_eq!(child0.as_slice(), &[0]);
        assert!(child0.starts_with(&root));

        let child0_1 = child0.child(1);
        assert_eq!(child0_1.as_slice(), &[0, 1]);
        assert!(child0_1.starts_with(&child0));
        assert!(child0_1.starts_with(&root));

        let child1 = root.child(1);
        assert!(!child0_1.starts_with(&child1));
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

    struct MockResolver;
    impl StyleResolverPort for MockResolver {
        fn resolve_style(
            &self,
            _query: &ElementQuery,
        ) -> crate::features::styling::domain::ComputedStyle {
            crate::features::styling::domain::ComputedStyle::default()
        }
    }

    #[test]
    fn test_resolve_styles_interaction_propagation() {
        let child1 = VNode::new_text(
            TextContent::new("c1".to_string()),
            None,
            None,
            None,
            None,
            None,
        );
        let child2 = VNode::new_text(
            TextContent::new("c2".to_string()),
            None,
            None,
            None,
            None,
            None,
        );
        let root = VNode::new_flex(vec![child1, child2], None, None, None, None, None);

        let resolver = MockResolver;
        let interaction = InteractionContext::new(
            Some(NodePath::new(vec![0])),
            Some(NodePath::new(vec![0])),
            Some(NodePath::new(vec![1])),
            true,
        );

        let styled = root.resolve_styles(&resolver, Some(&interaction), None);
        assert_eq!(styled.path(), &NodePath::root());

        if let StyledNode::Flex { children, .. } = styled {
            assert_eq!(children.len(), 2);
            assert_eq!(children[0].path(), &NodePath::new(vec![0]));
            assert_eq!(children[1].path(), &NodePath::new(vec![1]));
        } else {
            panic!("Expected StyledNode::Flex");
        }
    }

    #[test]
    fn test_click_handlers_operations() {
        let mut handlers = ClickHandlers::new();
        assert!(handlers.is_empty());
        assert_eq!(handlers.get(&PointerButton::Left), None);

        handlers.insert(PointerButton::Left, UiAction::Exec("left_cmd".into()));
        assert!(!handlers.is_empty());
        assert_eq!(
            handlers.get(&PointerButton::Left),
            Some(&UiAction::Exec("left_cmd".into()))
        );
        assert_eq!(handlers.get(&PointerButton::Right), None);

        let single = ClickHandlers::from_single(UiAction::Exec("single_cmd".into()));
        assert_eq!(
            single.get(&PointerButton::Left),
            Some(&UiAction::Exec("single_cmd".into()))
        );
        assert_eq!(
            single.get(&PointerButton::Middle),
            Some(&UiAction::Exec("single_cmd".into()))
        );
        assert_eq!(
            single.get(&PointerButton::Right),
            Some(&UiAction::Exec("single_cmd".into()))
        );
        assert_eq!(single.get(&PointerButton::Side), None);
    }

    #[test]
    fn test_click_handlers_deserialization() {
        // Single command format
        let single_json = r#"{"Exec": "single_cmd"}"#;
        let single_handlers: ClickHandlers = serde_json::from_str(single_json).unwrap();
        assert_eq!(
            single_handlers.get(&PointerButton::Left),
            Some(&UiAction::Exec("single_cmd".into()))
        );
        assert_eq!(
            single_handlers.get(&PointerButton::Right),
            Some(&UiAction::Exec("single_cmd".into()))
        );

        // Map format with named and numeric keys
        let map_json = r#"{
            "left": {"Exec": "left_cmd"},
            "right": {"Exec": "right_cmd"},
            "middle": {"Exec": "mid_cmd"},
            "side": {"Exec": "side_cmd"},
            "276": {"Exec": "extra_cmd"}
        }"#;
        let map_handlers: ClickHandlers = serde_json::from_str(map_json).unwrap();
        assert_eq!(
            map_handlers.get(&PointerButton::Left),
            Some(&UiAction::Exec("left_cmd".into()))
        );
        assert_eq!(
            map_handlers.get(&PointerButton::Right),
            Some(&UiAction::Exec("right_cmd".into()))
        );
        assert_eq!(
            map_handlers.get(&PointerButton::Middle),
            Some(&UiAction::Exec("mid_cmd".into()))
        );
        assert_eq!(
            map_handlers.get(&PointerButton::Side),
            Some(&UiAction::Exec("side_cmd".into()))
        );
        assert_eq!(
            map_handlers.get(&PointerButton::Extra),
            Some(&UiAction::Exec("extra_cmd".into()))
        );
    }

    #[test]
    fn test_vnode_with_popup_and_style_resolution() {
        struct MockResolver;
        impl StyleResolverPort for MockResolver {
            fn resolve_style(&self, _query: &ElementQuery) -> ComputedStyle {
                ComputedStyle::default()
            }
        }

        let popup_content = VNode::new_text(
            TextContent::new("popup body".to_string()),
            None,
            None,
            None,
            None,
            None,
        );
        let button = VNode::new_text(
            TextContent::new("Open".to_string()),
            None,
            None,
            None,
            None,
            None,
        )
        .with_popup(Box::new(popup_content.clone()));

        assert!(button.popup().is_some());
        assert_eq!(button.popup().unwrap(), &popup_content);

        let resolver = MockResolver;
        let styled = button.resolve_styles(&resolver, None, None);
        assert!(styled.popup().is_some());
    }

    #[test]
    fn test_vnode_grid_creation_and_resolution() {
        struct MockResolver;
        impl StyleResolverPort for MockResolver {
            fn resolve_style(&self, _query: &ElementQuery) -> ComputedStyle {
                ComputedStyle::default()
            }
        }

        let child1 = VNode::new_text(
            TextContent::new("Item 1".to_string()),
            None,
            None,
            None,
            None,
            None,
        );
        let child2 = VNode::new_text(
            TextContent::new("Item 2".to_string()),
            None,
            None,
            None,
            None,
            None,
        );

        let grid_node = VNode::new_grid(vec![child1, child2], None, None, None, None, None);
        assert_eq!(grid_node.tag(), NodeTag::Grid);
        assert!(grid_node.tag().is_container());
        assert_eq!(grid_node.children().len(), 2);

        let resolver = MockResolver;
        let styled = grid_node.resolve_styles(&resolver, None, None);
        if let StyledNode::Grid { children, style, .. } = styled {
            assert_eq!(children.len(), 2);
            assert_eq!(style.display(), Some(crate::features::styling::domain::DisplayMode::Grid));
            assert_eq!(style.grid_auto_flow(), Some(crate::features::styling::domain::GridAutoFlow::Row));
        } else {
            panic!("Expected StyledNode::Grid");
        }
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
