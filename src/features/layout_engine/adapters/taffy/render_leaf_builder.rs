use crate::features::layout_engine::domain::{RenderNode, StyledNode, StyledPanel, StyledPopup};
use crate::features::vdom::domain::{NodeKey, NodePath};
use crate::shared::primitives::geometry::Rect;

/// What every laid-out node carries over unchanged from its styled node, whatever its kind: its
/// identity and the floating content it opens.
pub(super) struct Carried {
    pub(super) path: NodePath,
    pub(super) node_key: Option<NodeKey>,
    pub(super) tooltip: Option<Box<StyledNode>>,
    pub(super) popup: Option<StyledPopup>,
    pub(super) panel: Option<StyledPanel>,
}

impl Carried {
    /// Clones what `node` carries over.
    pub(super) fn from_node(node: &StyledNode) -> Self {
        Self {
            path: node.path().clone(),
            node_key: node.node_key().cloned(),
            tooltip: node.tooltip().cloned().map(Box::new),
            popup: node.popup().cloned(),
            panel: node.panel().cloned(),
        }
    }
}

/// `node` laid out at `rect` when it is text.
fn text_node(node: &StyledNode, rect: Rect) -> Option<RenderNode> {
    let StyledNode::Text {
        text,
        style,
        on_click,
        on_hover,
        ..
    } = node
    else {
        return None;
    };
    let carried = Carried::from_node(node);
    Some(RenderNode::Text {
        path: carried.path,
        node_key: carried.node_key,
        rect,
        text: text.clone(),
        style: style.clone(),
        on_click: on_click.clone(),
        on_hover: on_hover.clone(),
        tooltip: carried.tooltip,
        popup: carried.popup,
        panel: carried.panel,
    })
}

/// `node` laid out at `rect` when it is a progress bar.
fn progress_node(node: &StyledNode, rect: Rect) -> Option<RenderNode> {
    let StyledNode::Progress {
        value,
        orientation,
        style,
        on_click,
        on_hover,
        ..
    } = node
    else {
        return None;
    };
    let carried = Carried::from_node(node);
    Some(RenderNode::Progress {
        path: carried.path,
        node_key: carried.node_key,
        rect,
        value: *value,
        orientation: *orientation,
        style: style.clone(),
        on_click: on_click.clone(),
        on_hover: on_hover.clone(),
        tooltip: carried.tooltip,
        popup: carried.popup,
        panel: carried.panel,
    })
}

/// `node` laid out at `rect` when it is a rect.
fn rect_node(node: &StyledNode, rect: Rect) -> Option<RenderNode> {
    let StyledNode::Rect {
        style,
        on_click,
        on_hover,
        ..
    } = node
    else {
        return None;
    };
    let carried = Carried::from_node(node);
    Some(RenderNode::Rect {
        path: carried.path,
        node_key: carried.node_key,
        rect,
        style: style.clone(),
        on_click: on_click.clone(),
        on_hover: on_hover.clone(),
        tooltip: carried.tooltip,
        popup: carried.popup,
        panel: carried.panel,
    })
}

/// `node` laid out at `rect` when it is an image.
fn image_node(node: &StyledNode, rect: Rect) -> Option<RenderNode> {
    let StyledNode::Image {
        data, pixel_size, ..
    } = node
    else {
        return None;
    };
    let carried = Carried::from_node(node);
    Some(RenderNode::Image {
        path: carried.path,
        node_key: carried.node_key,
        rect,
        data: data.clone(),
        pixel_size: *pixel_size,
        tooltip: carried.tooltip,
        popup: carried.popup,
        panel: carried.panel,
    })
}

/// `node` laid out at `rect` when it embeds a module.
fn module_node(node: &StyledNode, rect: Rect) -> Option<RenderNode> {
    let StyledNode::Module {
        key,
        options,
        style,
        on_click,
        on_hover,
        ..
    } = node
    else {
        return None;
    };
    let carried = Carried::from_node(node);
    Some(RenderNode::Module {
        path: carried.path,
        node_key: carried.node_key,
        rect,
        key: key.clone(),
        options: options.clone(),
        style: style.clone(),
        on_click: on_click.clone(),
        on_hover: on_hover.clone(),
        tooltip: carried.tooltip,
        popup: carried.popup,
        panel: carried.panel,
    })
}

pub(super) fn build_leaf_render_node(node: &StyledNode, rect: Rect) -> Option<RenderNode> {
    text_node(node, rect)
        .or_else(|| progress_node(node, rect))
        .or_else(|| rect_node(node, rect))
        .or_else(|| image_node(node, rect))
        .or_else(|| module_node(node, rect))
}
