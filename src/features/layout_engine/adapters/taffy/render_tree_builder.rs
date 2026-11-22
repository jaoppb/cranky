use crate::features::layout_engine::domain::{LayoutError, RenderNode, StyledNode};
use crate::shared::primitives::geometry::{Position, Rect, Size};
use taffy::TaffyTree;
use taffy::tree::NodeId;

use super::render_leaf_builder::build_leaf_render_node;
use crate::utils::{f32_to_i32, f32_to_u32};

/// One node of a computed taffy tree: the tree, and the id of the node in it.
#[derive(Debug, Clone, Copy)]
pub(super) struct TaffyNode<'a> {
    tree: &'a TaffyTree,
    id: NodeId,
}

impl<'a> TaffyNode<'a> {
    pub(super) const fn new(tree: &'a TaffyTree, id: NodeId) -> Self {
        Self { tree, id }
    }
}

fn build_children(
    parent: TaffyNode<'_>,
    children: &[StyledNode],
    pos: Position,
) -> Result<Vec<RenderNode>, LayoutError> {
    let child_ids = parent
        .tree
        .children(parent.id)
        .map_err(|error| LayoutError::EngineError(error.to_string()))?;
    let mut render_children = Vec::new();
    for (child, &child_id) in children.iter().zip(child_ids.iter()) {
        render_children.push(build_render_tree(
            TaffyNode::new(parent.tree, child_id),
            child,
            pos,
        )?);
    }
    Ok(render_children)
}

pub(super) fn build_render_tree(
    laid_out: TaffyNode<'_>,
    node: &StyledNode,
    offset: Position,
) -> Result<RenderNode, LayoutError> {
    let layout = laid_out
        .tree
        .layout(laid_out.id)
        .map_err(|error| LayoutError::EngineError(error.to_string()))?;

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
            children: build_children(laid_out, children, Position::new(abs_x, abs_y))?,
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
            children: build_children(laid_out, children, Position::new(abs_x, abs_y))?,
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
