//! Fixtures for the `diff_nodes` tests: the expected `UpdateProps` and `UpdateText` patches and
//! a text node with chosen selectors and interactions.
use crate::features::vdom::domain::{
    ElementSelectors, Interactions, NodeId, Patch, TextContent, VNode,
};

/// The one prop flag an `UpdateProps` patch reports changed.
#[derive(PartialEq, Eq)]
pub(super) enum Flag {
    Class,
    Identifier,
    Handlers,
}

/// What an `UpdateProps` patch reports besides its node id, all unchanged by default.
#[derive(Default)]
pub(super) struct PropsChange {
    pub(super) flag: Option<Flag>,
    pub(super) tooltip_patch: Option<Box<Patch>>,
    pub(super) popup_patch: Option<Box<Patch>>,
    pub(super) panel_patch: Option<Box<Patch>>,
}

/// The `UpdateProps` patch for `node_id` carrying `change` around `kind_patch`.
pub(super) fn update_props(node_id: NodeId, change: PropsChange, kind_patch: Patch) -> Patch {
    Patch::UpdateProps {
        node_id,
        class_changed: change.flag == Some(Flag::Class),
        id_changed: change.flag == Some(Flag::Identifier),
        handlers_changed: change.flag == Some(Flag::Handlers),
        tooltip_patch: change.tooltip_patch,
        popup_patch: change.popup_patch,
        panel_patch: change.panel_patch,
        kind_patch: Box::new(kind_patch),
    }
}

/// A text node showing `content` with `selectors` and `interactions`.
pub(super) fn styled(
    content: &str,
    selectors: ElementSelectors,
    interactions: Interactions,
) -> VNode {
    VNode::new_text(
        TextContent::new(content.to_string()),
        selectors,
        interactions,
    )
}

/// The `UpdateText` patch that shows `content` on `node`.
pub(super) fn update_text(node: &VNode, content: &str) -> Patch {
    Patch::UpdateText {
        node_id: node.node_id(),
        new_text: TextContent::new(content.to_string()),
    }
}
