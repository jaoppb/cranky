use crate::features::layout_engine::domain::{LayoutError, RenderNode, StyledNode};
use crate::shared::primitives::geometry::{Position, Rect, Size};
use taffy::TaffyTree;
use taffy::tree::NodeId;

use super::render_leaf_builder::build_leaf_render_node;
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
        Size::new(
            f32_to_u32(layout.size.width),
            f32_to_u32(layout.size.height),
        ),
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
        _ => Err(LayoutError::EngineError(
            "Unexpected container node".to_string(),
        )),
    }
}
