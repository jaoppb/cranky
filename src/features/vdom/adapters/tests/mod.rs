//! Tests that pin the exact patches, and their order, the differ emits for each kind of change,
//! and the logs it writes.
mod children_logs;
mod floating;
mod kinds;
mod nodes;
mod props;

use super::DefaultVdomDiffAdapter;
use crate::features::vdom::domain::{
    ElementSelectors, Interactions, NodeKey, Patch, TextContent, VNode,
};

/// A plain text node showing `content`, with no class, id, handler or tooltip.
fn text(content: &str) -> VNode {
    VNode::new_text(
        TextContent::new(content.to_string()),
        ElementSelectors::default(),
        Interactions::default(),
    )
}

/// A text node showing `content`, keyed by `key`.
fn keyed(key: &str, content: &str) -> VNode {
    text(content).with_key(NodeKey::new(key).unwrap())
}

/// A flex container holding `children`.
fn flex(children: Vec<VNode>) -> VNode {
    VNode::new_flex(
        children,
        ElementSelectors::default(),
        Interactions::default(),
    )
}

/// The patch the default differ produces from `old` to `new`.
fn diff(old: &VNode, new: &VNode) -> Patch {
    DefaultVdomDiffAdapter::new().diff_nodes(old, new)
}
