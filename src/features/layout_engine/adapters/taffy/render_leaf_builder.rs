use crate::features::layout_engine::domain::{RenderNode, StyledNode};
use crate::shared::primitives::geometry::Rect;

fn build_content_node(node: &StyledNode, rect: Rect) -> Option<RenderNode> {
    match node {
        StyledNode::Text {
            path, node_key, text, style, on_click, on_hover, tooltip, popup, panel,
        } => Some(RenderNode::Text {
            path: path.clone(), node_key: node_key.clone(), rect, text: text.clone(), style: style.clone(),
            on_click: on_click.clone(), on_hover: on_hover.clone(),
            tooltip: tooltip.clone(), popup: popup.clone(), panel: panel.clone(),
        }),
        StyledNode::Progress {
            path, node_key, value, orientation, style, on_click, on_hover, tooltip, popup, panel,
        } => Some(RenderNode::Progress {
            path: path.clone(), node_key: node_key.clone(), rect, value: *value, orientation: *orientation, style: style.clone(),
            on_click: on_click.clone(), on_hover: on_hover.clone(),
            tooltip: tooltip.clone(), popup: popup.clone(), panel: panel.clone(),
        }),
        _ => None,
    }
}

fn build_atomic_node(node: &StyledNode, rect: Rect) -> Option<RenderNode> {
    match node {
        StyledNode::Rect {
            path, node_key, style, on_click, on_hover, tooltip, popup, panel,
        } => Some(RenderNode::Rect {
            path: path.clone(), node_key: node_key.clone(), rect, style: style.clone(),
            on_click: on_click.clone(), on_hover: on_hover.clone(),
            tooltip: tooltip.clone(), popup: popup.clone(), panel: panel.clone(),
        }),
        StyledNode::Image {
            path, node_key, data, pixel_size, tooltip, popup, panel, ..
        } => Some(RenderNode::Image {
            path: path.clone(), node_key: node_key.clone(), rect, data: data.clone(), pixel_size: *pixel_size,
            tooltip: tooltip.clone(), popup: popup.clone(), panel: panel.clone(),
        }),
        StyledNode::Module {
            path, node_key, key, options, style, on_click, on_hover, tooltip, popup, panel,
        } => Some(RenderNode::Module {
            path: path.clone(), node_key: node_key.clone(), rect, key: key.clone(), options: options.clone(), style: style.clone(),
            on_click: on_click.clone(), on_hover: on_hover.clone(),
            tooltip: tooltip.clone(), popup: popup.clone(), panel: panel.clone(),
        }),
        _ => None,
    }
}

pub(super) fn build_leaf_render_node(node: &StyledNode, rect: Rect) -> Option<RenderNode> {
    build_content_node(node, rect).or_else(|| build_atomic_node(node, rect))
}
