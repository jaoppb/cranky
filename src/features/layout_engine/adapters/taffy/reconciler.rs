use super::style_builder::node_to_style;
use super::tree_builder::TaffyTreeBuilder;
use crate::features::layout_engine::domain::{LayoutError, StyledNode, TextMeasurer};
use taffy::style::Style;
use taffy::tree::NodeId;

#[derive(Clone)]
pub struct LayoutState {
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
pub struct ChildrenPatch<'a> {
    pub patches: Vec<Patch<'a>>,
    pub removed: Vec<NodeId>,
}

pub enum Patch<'a> {
    Keep(&'a LayoutState),
    Update {
        old_state: &'a LayoutState,
        new_layout: &'a StyledNode,
        style: Box<Option<Style>>,
        children: Option<ChildrenPatch<'a>>,
    },
    Replace {
        old_state: &'a LayoutState,
        new_layout: &'a StyledNode,
    },
    Create {
        new_layout: &'a StyledNode,
    },
}

pub(super) fn build_layout_state(
    builder: &mut TaffyTreeBuilder,
    node: &StyledNode,
    measurer: &mut dyn TextMeasurer,
) -> Result<LayoutState, LayoutError> {
    let style = node_to_style(node, measurer);

    if let StyledNode::Flex { children, .. } | StyledNode::Grid { children, .. } = node {
        let mut state_children = Vec::new();
        let mut child_ids = Vec::new();
        for child in children {
            let state_child = build_layout_state(builder, child, measurer)?;
            child_ids.push(state_child.root_node);
            state_children.push(state_child);
        }

        let node_id = builder.add_node(style, &child_ids)?;

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

pub(super) fn apply_patch(
    builder: &mut TaffyTreeBuilder,
    patch: Patch,
    measurer: &mut dyn TextMeasurer,
) -> Result<LayoutState, LayoutError> {
    match patch {
        Patch::Keep(state) => Ok(state.clone()),
        Patch::Update {
            old_state,
            new_layout,
            style,
            children,
        } => {
            let node_id = old_state.root_node;

            if let Some(s) = *style {
                builder.set_style(node_id, s)?;
            }

            // `Some(_)` is an authoritative child list, even when empty;
            // `None` means the differ never looked at children (leaf or Module).
            let had_child_list = children.is_some();
            let mut new_state_children = Vec::new();
            if let Some(ChildrenPatch { patches, removed }) = children {
                let mut new_child_ids = Vec::with_capacity(patches.len());
                for cp in patches {
                    let child_state = apply_patch(builder, cp, measurer)?;
                    new_child_ids.push(child_state.root_node);
                    new_state_children.push(child_state);
                }

                // `removed` is exactly the old children absent from the new
                // list (by key, or by trailing position when unkeyed) —
                // freed here, before `new_state_children` below replaces
                // `old_state.children` for good.
                for dead_id in removed {
                    builder.remove_recursive(dead_id);
                }

                builder.set_children(node_id, &new_child_ids)?;
            }

            Ok(LayoutState {
                root_node: node_id,
                layout: new_layout.clone(),
                // Never carry `old_state.children` across an authoritative
                // child list: the loop above already freed every one of their
                // taffy nodes, and a freed NodeId handed to the next frame's
                // `remove_recursive` panics inside taffy's SlotMap rather than
                // returning an error.
                children: if had_child_list {
                    new_state_children
                } else {
                    old_state.children.clone()
                },
            })
        }
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
