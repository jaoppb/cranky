mod children;
#[cfg(test)]
mod tests;

use super::reconciler::{LayoutState, Patch};
use super::style_builder::node_to_style;
use crate::features::layout_engine::domain::{StyledNode, TextMeasurer};
use children::diff_container;

/// Diffs two leaves of the same kind: kept when nothing that sizes or styles
/// them changed, updated in place with a fresh style otherwise. `None` for a
/// container or a module.
fn diff_leaf<'a>(
    old_state: &'a LayoutState,
    new_layout: &'a StyledNode,
    measurer: &mut dyn TextMeasurer,
) -> Option<Patch<'a>> {
    let is_same = match (&old_state.layout, new_layout) {
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
        ) => old_text == new_text && old_style == new_style,
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
        ) => old_val == new_val && old_orient == new_orient && old_style == new_style,
        (
            StyledNode::Rect {
                style: old_style, ..
            },
            StyledNode::Rect {
                style: new_style, ..
            },
        ) => old_style == new_style,
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
        ) => old_size == new_size && old_style == new_style,
        _ => return None,
    };
    if is_same {
        return Some(Patch::Keep(old_state));
    }
    let style = node_to_style(new_layout, measurer);
    Some(Patch::Update {
        old_state,
        new_layout,
        style: Box::new(Some(style)),
        children: None,
    })
}

#[must_use]
pub(super) fn diff<'a>(
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
