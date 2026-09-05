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

pub enum Patch<'a> {
    Keep(&'a LayoutState),
    Update {
        old_state: &'a LayoutState,
        new_layout: &'a StyledNode,
        style: Box<Option<Style>>,
        children: Option<Vec<Self>>,
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

            let mut new_state_children = Vec::new();
            if let Some(child_patches) = children {
                let mut new_child_ids = Vec::new();
                for cp in child_patches {
                    let child_state = apply_patch(builder, cp, measurer)?;
                    new_child_ids.push(child_state.root_node);
                    new_state_children.push(child_state);
                }

                if old_state.children.len() > new_state_children.len() {
                    for child in old_state.children.iter().skip(new_state_children.len()) {
                        builder.remove_recursive(child.root_node);
                    }
                }

                builder.set_children(node_id, &new_child_ids)?;
            }

            Ok(LayoutState {
                root_node: node_id,
                layout: new_layout.clone(),
                children: if new_state_children.is_empty() && !old_state.children.is_empty() {
                    old_state.children.clone()
                } else {
                    new_state_children
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
