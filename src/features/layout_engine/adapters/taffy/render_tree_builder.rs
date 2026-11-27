use crate::features::layout_engine::domain::{LayoutError, RenderNode, StyledNode};
use crate::shared::primitives::geometry::{Position, Rect, Size};
use taffy::TaffyTree;
use taffy::tree::NodeId;

use super::render_leaf_builder::{Carried, build_leaf_render_node};
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
    let rect = laid_out_rect(laid_out, offset)?;
    if let Some(leaf) = build_leaf_render_node(node, rect) {
        return Ok(leaf);
    }
    container_node(laid_out, node, rect)
}

/// The box taffy laid `laid_out` out in, moved by `offset`.
fn laid_out_rect(laid_out: TaffyNode<'_>, offset: Position) -> Result<Rect, LayoutError> {
    let layout = laid_out
        .tree
        .layout(laid_out.id)
        .map_err(|error| LayoutError::EngineError(error.to_string()))?;
    let abs_x = offset.x().saturating_add(f32_to_i32(layout.location.x));
    let abs_y = offset.y().saturating_add(f32_to_i32(layout.location.y));
    Ok(Rect::new(
        Position::new(abs_x, abs_y),
        Size::new(
            f32_to_u32(layout.size.width),
            f32_to_u32(layout.size.height),
        ),
    ))
}

/// `node` laid out at `rect` with its children, which are placed from `rect`'s position; an
/// error when `node` is not a flex or grid container.
fn container_node(
    laid_out: TaffyNode<'_>,
    node: &StyledNode,
    rect: Rect,
) -> Result<RenderNode, LayoutError> {
    let unexpected = || LayoutError::EngineError("Unexpected container node".to_string());
    if !matches!(node, StyledNode::Flex { .. } | StyledNode::Grid { .. }) {
        return Err(unexpected());
    }
    let children = build_children(laid_out, node.children(), Position::new(rect.x(), rect.y()))?;
    match node {
        StyledNode::Flex { .. } => flex_node(node, rect, children),
        _ => grid_node(node, rect, children),
    }
    .ok_or_else(unexpected)
}

/// `node` laid out at `rect` over `children` when it is a flex container.
fn flex_node(node: &StyledNode, rect: Rect, children: Vec<RenderNode>) -> Option<RenderNode> {
    let StyledNode::Flex {
        style,
        on_click,
        on_hover,
        ..
    } = node
    else {
        return None;
    };
    let carried = Carried::from_node(node);
    Some(RenderNode::Flex {
        path: carried.path,
        node_key: carried.node_key,
        rect,
        children,
        style: style.clone(),
        on_click: on_click.clone(),
        on_hover: on_hover.clone(),
        tooltip: carried.tooltip,
        popup: carried.popup,
        panel: carried.panel,
    })
}

/// `node` laid out at `rect` over `children` when it is a grid container.
fn grid_node(node: &StyledNode, rect: Rect, children: Vec<RenderNode>) -> Option<RenderNode> {
    let StyledNode::Grid {
        style,
        on_click,
        on_hover,
        ..
    } = node
    else {
        return None;
    };
    let carried = Carried::from_node(node);
    Some(RenderNode::Grid {
        path: carried.path,
        node_key: carried.node_key,
        rect,
        children,
        style: style.clone(),
        on_click: on_click.clone(),
        on_hover: on_hover.clone(),
        tooltip: carried.tooltip,
        popup: carried.popup,
        panel: carried.panel,
    })
}
