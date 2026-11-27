mod children;
#[cfg(test)]
mod tests;

use super::reconciler::{LayoutState, Patch, Update};
use super::style_builder::node_to_style;
use crate::features::layout_engine::domain::{StyledNode, TextMeasurer};
use crate::features::styling::domain::{Orientation, ProgressValue};
use crate::features::vdom::domain::TextContent;
use crate::shared::primitives::geometry::Size;
use children::diff_container;

/// What sizes a leaf besides its style: its text, its value and orientation, or its pixel size.
#[derive(PartialEq)]
enum LeafContent<'a> {
    Text(&'a TextContent),
    Progress(&'a ProgressValue, &'a Orientation),
    Rect,
    Image(&'a Size),
}

/// The content of `node`, or `None` for a container or a module.
const fn leaf_content(node: &StyledNode) -> Option<LeafContent<'_>> {
    match node {
        StyledNode::Text { text, .. } => Some(LeafContent::Text(text)),
        StyledNode::Progress {
            value, orientation, ..
        } => Some(LeafContent::Progress(value, orientation)),
        StyledNode::Rect { .. } => Some(LeafContent::Rect),
        StyledNode::Image { pixel_size, .. } => Some(LeafContent::Image(pixel_size)),
        StyledNode::Flex { .. } | StyledNode::Grid { .. } | StyledNode::Module { .. } => None,
    }
}

/// Diffs two leaves of the same kind: kept when nothing that sizes or styles
/// them changed, updated in place with a fresh style otherwise. `None` for a
/// container or a module, and for two leaves of different kinds, which `diff`
/// has already replaced before asking.
fn diff_leaf<'a>(
    old_state: &'a LayoutState,
    new_layout: &'a StyledNode,
    measurer: &mut dyn TextMeasurer,
) -> Option<Patch<'a>> {
    let (Some(old_content), Some(new_content)) =
        (leaf_content(&old_state.layout), leaf_content(new_layout))
    else {
        return None;
    };
    if std::mem::discriminant(&old_content) != std::mem::discriminant(&new_content) {
        return None;
    }
    if old_content == new_content && old_state.layout.style() == new_layout.style() {
        return Some(Patch::Keep(old_state));
    }
    Some(restyle(old_state, new_layout, measurer))
}

/// Updates `old_state`'s node in place with `new_layout`'s fresh style, leaving its children
/// alone.
fn restyle<'a>(
    old_state: &'a LayoutState,
    new_layout: &'a StyledNode,
    measurer: &mut dyn TextMeasurer,
) -> Patch<'a> {
    let style = node_to_style(new_layout, measurer);
    Patch::Update(Update {
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
        (StyledNode::Flex { .. }, StyledNode::Flex { .. })
        | (StyledNode::Grid { .. }, StyledNode::Grid { .. }) => {
            diff_container(old_state, new_layout, measurer)
        }
        (StyledNode::Module { .. }, StyledNode::Module { .. }) => {
            restyle(old_state, new_layout, measurer)
        }
        _ => Patch::Replace {
            old_state,
            new_layout,
        },
    }
}
