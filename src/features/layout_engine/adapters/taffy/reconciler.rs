use super::style_builder::node_to_style;
use super::tree_builder::TaffyTreeBuilder;
use crate::features::layout_engine::domain::{LayoutError, StyledNode, TextMeasurer};
use taffy::style::Style;
use taffy::tree::NodeId;

#[derive(Clone)]
pub(super) struct LayoutState {
    pub(super) root_node: NodeId,
    pub(super) layout: StyledNode,
    pub(super) children: Vec<Self>,
}

/// A container's child reconciliation.
///
/// `patches` applied in the order given (which becomes the new taffy child
/// order — `apply_patch` never reorders on its own, so matching by key
/// rather than position is what lets a container's own order change), plus
/// every old child `NodeId` that has no counterpart in the new list and must
/// be freed.
///
/// Computed once at diff time (positionally or by `NodeKey` — see
/// `super::diff::children`) so `apply_patch` never has to guess which old
/// children survived; guessing via "index >= new length" is exactly what
/// let dead `NodeId`s slip back into `LayoutState` before.
pub(super) struct ChildrenPatch<'a> {
    pub(crate) patches: Vec<Patch<'a>>,
    pub(crate) removed: Vec<NodeId>,
}

pub(super) enum Patch<'a> {
    Keep(&'a LayoutState),
    Update(Update<'a>),
    Replace {
        old_state: &'a LayoutState,
        new_layout: &'a StyledNode,
    },
    Create {
        new_layout: &'a StyledNode,
    },
}

/// A node kept in place and updated to `new_layout`: restyled when `style` holds a new style,
/// and given a new child list when `children` holds one.
pub(super) struct Update<'a> {
    pub(super) old_state: &'a LayoutState,
    pub(super) new_layout: &'a StyledNode,
    pub(super) style: Box<Option<Style>>,
    /// `Some(_)` is an authoritative child list, even when empty; `None` means the differ never
    /// looked at children (leaf or Module).
    pub(super) children: Option<ChildrenPatch<'a>>,
}

/// The taffy nodes of `states`, in order.
fn root_nodes(states: &[LayoutState]) -> Vec<NodeId> {
    states.iter().map(|state| state.root_node).collect()
}

pub(super) fn build_layout_state(
    builder: &mut TaffyTreeBuilder<'_>,
    node: &StyledNode,
    measurer: &mut dyn TextMeasurer,
) -> Result<LayoutState, LayoutError> {
    let style = node_to_style(node, measurer);

    if let StyledNode::Flex { children, .. } | StyledNode::Grid { children, .. } = node {
        let state_children = build_children(builder, children, measurer)?;
        let node_id = builder.add_node(style, &root_nodes(&state_children))?;
        Ok(LayoutState {
            root_node: node_id,
            layout: node.clone(),
            children: state_children,
        })
    } else {
        let node_id = builder.add_leaf(style)?;
        Ok(LayoutState {
            root_node: node_id,
            layout: node.clone(),
            children: Vec::new(),
        })
    }
}

/// Builds the taffy subtree of each of `children`, in order.
fn build_children(
    builder: &mut TaffyTreeBuilder<'_>,
    children: &[StyledNode],
    measurer: &mut dyn TextMeasurer,
) -> Result<Vec<LayoutState>, LayoutError> {
    let mut state_children = Vec::with_capacity(children.len());
    for child in children {
        state_children.push(build_layout_state(builder, child, measurer)?);
    }
    Ok(state_children)
}

pub(super) fn apply_patch(
    builder: &mut TaffyTreeBuilder<'_>,
    patch: Patch<'_>,
    measurer: &mut dyn TextMeasurer,
) -> Result<LayoutState, LayoutError> {
    match patch {
        Patch::Keep(state) => Ok(state.clone()),
        Patch::Update(update) => apply_update(builder, update, measurer),
        Patch::Replace {
            old_state,
            new_layout,
        } => {
            let new_state = build_layout_state(builder, new_layout, measurer)?;
            builder.remove_recursive(old_state.root_node);
            Ok(new_state)
        }
        Patch::Create { new_layout } => build_layout_state(builder, new_layout, measurer),
    }
}

/// Restyles the kept node and, given an authoritative child list, applies it and makes it the
/// node's children.
fn apply_update(
    builder: &mut TaffyTreeBuilder<'_>,
    update: Update<'_>,
    measurer: &mut dyn TextMeasurer,
) -> Result<LayoutState, LayoutError> {
    let Update {
        old_state,
        new_layout,
        style,
        children,
    } = update;
    let node_id = old_state.root_node;
    if let Some(new_style) = *style {
        builder.set_style(node_id, new_style)?;
    }
    // Never carry `old_state.children` across an authoritative child list:
    // applying it already freed every one of their taffy nodes, and a freed
    // NodeId handed to the next frame's `remove_recursive` panics inside
    // taffy's SlotMap rather than returning an error.
    let children = if let Some(children_patch) = children {
        let new_children = apply_children_patch(builder, children_patch, measurer)?;
        builder.set_children(node_id, &root_nodes(&new_children))?;
        new_children
    } else {
        old_state.children.clone()
    };
    Ok(LayoutState {
        root_node: node_id,
        layout: new_layout.clone(),
        children,
    })
}

/// Applies each child patch in order, then frees the removed children, and returns the new
/// children's states.
fn apply_children_patch(
    builder: &mut TaffyTreeBuilder<'_>,
    children_patch: ChildrenPatch<'_>,
    measurer: &mut dyn TextMeasurer,
) -> Result<Vec<LayoutState>, LayoutError> {
    let ChildrenPatch { patches, removed } = children_patch;
    let mut new_children = Vec::with_capacity(patches.len());
    for child_patch in patches {
        new_children.push(apply_patch(builder, child_patch, measurer)?);
    }
    // `removed` is exactly the old children absent from the new list (by
    // key, or by trailing position when unkeyed) — freed here, before the
    // new children replace `old_state.children` for good.
    for dead_id in removed {
        builder.remove_recursive(dead_id);
    }
    Ok(new_children)
}
