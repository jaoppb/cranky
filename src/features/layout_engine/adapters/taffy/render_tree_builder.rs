use crate::features::layout_engine::domain::{LayoutError, RenderNode, StyledNode};
use crate::shared::primitives::geometry::{Position, Rect, Size};
use taffy::tree::NodeId;
use taffy::TaffyTree;

use crate::utils::{f32_to_i32, f32_to_u32};

fn build_children(
    taffy: &TaffyTree,
    node_id: NodeId,
    children: &[StyledNode],
    pos: Position,
) -> Result<Vec<RenderNode>, LayoutError> {
    let child_ids = taffy
        .children(node_id)
        .map_err(|e| LayoutError::EngineError(e.to_string()))?;
    let mut render_children = Vec::new();
    for (child, &child_id) in children.iter().zip(child_ids.iter()) {
        render_children.push(build_render_tree(taffy, child_id, child, pos)?);
    }
    Ok(render_children)
}

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
            path, node_key, key, style, on_click, on_hover, tooltip, popup, panel, ..
        } => Some(RenderNode::Module {
            path: path.clone(), node_key: node_key.clone(), rect, key: key.clone(), style: style.clone(),
            on_click: on_click.clone(), on_hover: on_hover.clone(),
            tooltip: tooltip.clone(), popup: popup.clone(), panel: panel.clone(),
        }),
        _ => None,
    }
}

fn build_leaf_render_node(node: &StyledNode, rect: Rect) -> Option<RenderNode> {
    build_content_node(node, rect).or_else(|| build_atomic_node(node, rect))
}

pub(super) fn build_render_tree(
    taffy: &TaffyTree,
    node_id: NodeId,
    node: &StyledNode,
    offset: Position,
) -> Result<RenderNode, LayoutError> {
    let layout = taffy
        .layout(node_id)
        .map_err(|e| LayoutError::EngineError(e.to_string()))?;

    let abs_x = offset.x().saturating_add(f32_to_i32(layout.location.x));
    let abs_y = offset.y().saturating_add(f32_to_i32(layout.location.y));
    let rect = Rect::new(
        Position::new(abs_x, abs_y),
        Size::new(f32_to_u32(layout.size.width), f32_to_u32(layout.size.height)),
    );

    if let Some(leaf) = build_leaf_render_node(node, rect) {
        return Ok(leaf);
    }

    match node {
        StyledNode::Flex {
            path,
            node_key,
            children,
            style,
            on_click,
            on_hover,
            tooltip,
            popup,
            panel,
        } => Ok(RenderNode::Flex {
            path: path.clone(),
            node_key: node_key.clone(),
            rect,
            children: build_children(taffy, node_id, children, Position::new(abs_x, abs_y))?,
            style: style.clone(),
            on_click: on_click.clone(),
            on_hover: on_hover.clone(),
            tooltip: tooltip.clone(),
            popup: popup.clone(),
            panel: panel.clone(),
        }),
        StyledNode::Grid {
            path,
            node_key,
            children,
            style,
            on_click,
            on_hover,
            tooltip,
            popup,
            panel,
        } => Ok(RenderNode::Grid {
            path: path.clone(),
            node_key: node_key.clone(),
            rect,
            children: build_children(taffy, node_id, children, Position::new(abs_x, abs_y))?,
            style: style.clone(),
            on_click: on_click.clone(),
            on_hover: on_hover.clone(),
            tooltip: tooltip.clone(),
            popup: popup.clone(),
            panel: panel.clone(),
        }),
        _ => Err(LayoutError::EngineError("Unexpected container node".to_string())),
    }
}
