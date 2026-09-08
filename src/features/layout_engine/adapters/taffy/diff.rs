use super::reconciler::{LayoutState, Patch};
use super::style_builder::node_to_style;
use crate::features::layout_engine::domain::{StyledNode, TextMeasurer};
use crate::features::styling::domain::ComputedStyle;

fn diff_container<'a>(
    old_state: &'a LayoutState,
    new_layout: &'a StyledNode,
    old_style: &ComputedStyle,
    new_style: &ComputedStyle,
    new_children: &'a [StyledNode],
    measurer: &mut dyn TextMeasurer,
) -> Patch<'a> {
    let style = if old_style == new_style {
        None
    } else {
        Some(node_to_style(new_layout, measurer))
    };

    let mut child_patches = Vec::with_capacity(new_children.len());
    for (i, new_child) in new_children.iter().enumerate() {
        if let Some(old_child) = old_state.children.get(i) {
            child_patches.push(diff(old_child, new_child, measurer));
        } else {
            child_patches.push(Patch::Create {
                new_layout: new_child,
            });
        }
    }

    Patch::Update {
        old_state,
        new_layout,
        style: Box::new(style),
        children: Some(child_patches),
    }
}

fn leaf_patch<'a>(
    old_state: &'a LayoutState,
    new_layout: &'a StyledNode,
    measurer: &mut dyn TextMeasurer,
    is_same: bool,
) -> Patch<'a> {
    if is_same {
        Patch::Keep(old_state)
    } else {
        let style = node_to_style(new_layout, measurer);
        Patch::Update {
            old_state,
            new_layout,
            style: Box::new(Some(style)),
            children: None,
        }
    }
}

fn diff_leaf<'a>(
    old_state: &'a LayoutState,
    new_layout: &'a StyledNode,
    measurer: &mut dyn TextMeasurer,
) -> Option<Patch<'a>> {
    match (&old_state.layout, new_layout) {
        (
            StyledNode::Text {
                text: old_text,
                style: old_style,
                ..
            },
            StyledNode::Text {
                text: new_text,
                style: new_style,
                ..
            },
        ) => Some(leaf_patch(
            old_state,
            new_layout,
            measurer,
            old_text == new_text && old_style == new_style,
        )),
        (
            StyledNode::Progress {
                value: old_val,
                orientation: old_orient,
                style: old_style,
                ..
            },
            StyledNode::Progress {
                value: new_val,
                orientation: new_orient,
                style: new_style,
                ..
            },
        ) => Some(leaf_patch(
            old_state,
            new_layout,
            measurer,
            old_val == new_val && old_orient == new_orient && old_style == new_style,
        )),
        (
            StyledNode::Rect {
                style: old_style, ..
            },
            StyledNode::Rect {
                style: new_style, ..
            },
        ) => Some(leaf_patch(old_state, new_layout, measurer, old_style == new_style)),
        (
            StyledNode::Image {
                pixel_size: old_size,
                style: old_style,
                ..
            },
            StyledNode::Image {
                pixel_size: new_size,
                style: new_style,
                ..
            },
        ) => Some(leaf_patch(
            old_state,
            new_layout,
            measurer,
            old_size == new_size && old_style == new_style,
        )),
        _ => None,
    }
}

#[must_use]
pub fn diff<'a>(
    old_state: &'a LayoutState,
    new_layout: &'a StyledNode,
    measurer: &mut dyn TextMeasurer,
) -> Patch<'a> {
    if std::mem::discriminant(&old_state.layout) != std::mem::discriminant(new_layout) {
        return Patch::Replace {
            old_state,
            new_layout,
        };
    }

    if let Some(patch) = diff_leaf(old_state, new_layout, measurer) {
        return patch;
    }

    match (&old_state.layout, new_layout) {
        (
            StyledNode::Flex {
                style: old_style, ..
            },
            StyledNode::Flex {
                style: new_style,
                children: new_children,
                ..
            },
        )
        | (
            StyledNode::Grid {
                style: old_style, ..
            },
            StyledNode::Grid {
                style: new_style,
                children: new_children,
                ..
            },
        ) => diff_container(
            old_state,
            new_layout,
            old_style,
            new_style,
            new_children,
            measurer,
        ),
        (StyledNode::Module { .. }, StyledNode::Module { .. }) => {
            let style = node_to_style(new_layout, measurer);
            Patch::Update {
                old_state,
                new_layout,
                style: Box::new(Some(style)),
                children: None,
            }
        }
        _ => Patch::Replace {
            old_state,
            new_layout,
        },
    }
}
