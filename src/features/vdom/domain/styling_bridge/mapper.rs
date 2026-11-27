use crate::features::layout_engine::domain::{StyledNode, StyledPanel, StyledPopup};
use crate::features::styling::domain::{ComputedStyle, Orientation, ProgressValue};
use crate::features::vdom::domain::{
    ClickHandlers, NodeKey, NodePath, TextContent, UiAction, VNode, VNodeKind,
};
use crate::shared::primitives::geometry::Size;
use crate::shared::primitives::{BinaryData, ModuleKey, ModuleOptions};

/// What the styling pass resolved for one node, beside what the node itself carries: its computed
/// style, its styled tooltip, popup and panel, and its styled children.
pub(super) struct StyledParts {
    pub(super) style: ComputedStyle,
    pub(super) floating: StyledFloating,
    pub(super) children: Vec<StyledNode>,
}

/// A node's styled tooltip, popup and panel, each resolved on its own surface.
pub(super) struct StyledFloating {
    pub(super) tooltip: Option<Box<StyledNode>>,
    pub(super) popup: Option<StyledPopup>,
    pub(super) panel: Option<StyledPanel>,
}

/// What every styled variant carries besides its kind's own data: the node's path, key, style,
/// actions and styled floating parts. Image takes all but the actions.
struct NodeShell {
    path: NodePath,
    node_key: Option<NodeKey>,
    style: ComputedStyle,
    on_click: Option<ClickHandlers>,
    on_hover: Option<UiAction>,
    tooltip: Option<Box<StyledNode>>,
    popup: Option<StyledPopup>,
    panel: Option<StyledPanel>,
}

/// Builds the styled node for `vnode` at `path` from what the styling pass resolved for it, as the
/// `StyledNode` variant of its kind.
#[must_use]
pub(super) fn map_to_styled_node(vnode: &VNode, path: &NodePath, parts: StyledParts) -> StyledNode {
    let (shell, children) = NodeShell::split(vnode, path, parts);
    match vnode.kind() {
        VNodeKind::Flex { .. } => shell.into_flex(children),
        VNodeKind::Grid { .. } => shell.into_grid(children),
        VNodeKind::Text { text } => shell.into_text(text.clone()),
        VNodeKind::Progress { value, orientation } => shell.into_progress(*value, *orientation),
        VNodeKind::Rect => shell.into_rect(),
        VNodeKind::Image { data, pixel_size } => shell.into_image(data.clone(), *pixel_size),
        VNodeKind::Module {
            name,
            instance_id,
            options,
        } => shell.into_module(
            ModuleKey::new(name.clone(), instance_id.clone()),
            options.clone(),
        ),
    }
}

impl NodeShell {
    /// Splits `parts` into the shell of `vnode` at `path` and its styled children.
    fn split(vnode: &VNode, path: &NodePath, parts: StyledParts) -> (Self, Vec<StyledNode>) {
        let StyledParts {
            style,
            floating:
                StyledFloating {
                    tooltip,
                    popup,
                    panel,
                },
            children,
        } = parts;
        let shell = Self {
            path: path.clone(),
            node_key: vnode.key().cloned(),
            style,
            on_click: vnode.on_click().cloned(),
            on_hover: vnode.on_hover().cloned(),
            tooltip,
            popup,
            panel,
        };
        (shell, children)
    }

    fn into_flex(self, children: Vec<StyledNode>) -> StyledNode {
        StyledNode::Flex {
            path: self.path,
            node_key: self.node_key,
            children,
            style: self.style,
            on_click: self.on_click,
            on_hover: self.on_hover,
            tooltip: self.tooltip,
            popup: self.popup,
            panel: self.panel,
        }
    }

    fn into_grid(self, children: Vec<StyledNode>) -> StyledNode {
        StyledNode::Grid {
            path: self.path,
            node_key: self.node_key,
            children,
            style: self.style,
            on_click: self.on_click,
            on_hover: self.on_hover,
            tooltip: self.tooltip,
            popup: self.popup,
            panel: self.panel,
        }
    }

    fn into_text(self, text: TextContent) -> StyledNode {
        StyledNode::Text {
            path: self.path,
            node_key: self.node_key,
            text,
            style: self.style,
            on_click: self.on_click,
            on_hover: self.on_hover,
            tooltip: self.tooltip,
            popup: self.popup,
            panel: self.panel,
        }
    }

    fn into_progress(self, value: ProgressValue, orientation: Orientation) -> StyledNode {
        StyledNode::Progress {
            path: self.path,
            node_key: self.node_key,
            value,
            orientation,
            style: self.style,
            on_click: self.on_click,
            on_hover: self.on_hover,
            tooltip: self.tooltip,
            popup: self.popup,
            panel: self.panel,
        }
    }

    fn into_rect(self) -> StyledNode {
        StyledNode::Rect {
            path: self.path,
            node_key: self.node_key,
            style: self.style,
            on_click: self.on_click,
            on_hover: self.on_hover,
            tooltip: self.tooltip,
            popup: self.popup,
            panel: self.panel,
        }
    }

    /// An image reacts to no click or hover, so its actions are dropped.
    fn into_image(self, data: BinaryData, pixel_size: Size) -> StyledNode {
        StyledNode::Image {
            path: self.path,
            node_key: self.node_key,
            data,
            pixel_size,
            style: self.style,
            tooltip: self.tooltip,
            popup: self.popup,
            panel: self.panel,
        }
    }

    fn into_module(self, key: ModuleKey, options: ModuleOptions) -> StyledNode {
        StyledNode::Module {
            path: self.path,
            node_key: self.node_key,
            key,
            options,
            style: self.style,
            on_click: self.on_click,
            on_hover: self.on_hover,
            tooltip: self.tooltip,
            popup: self.popup,
            panel: self.panel,
        }
    }
}
