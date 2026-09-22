use crate::features::layout_engine::domain::RenderNode;
use crate::features::vdom::domain::{NodePath, NodeRef};

/// Captures a `NodeRef` for the deepest node in a hit-test chain (root to
/// leaf, as returned by `RenderNode::hit_test`), anchored at the nearest
/// keyed ancestor rather than at the leaf itself.
///
/// Using only the leaf (e.g. an unkeyed text label inside a keyed list
/// item) would strand the interaction on that item's key entirely, and
/// anchoring at the keyed ancestor alone would drop the leaf's own
/// identity — the anchor is only where the key chain is captured; the walk
/// from the anchor down to the actual hit leaf is what lets every node on
/// that chain (keyed item and unkeyed label both) match again later. See
/// `NodeRef`'s own docs for why the anchor is a key *chain*, not one key.
#[must_use]
pub(super) fn capture_node_ref(hit: &[&RenderNode]) -> Option<NodeRef> {
    let leaf = hit.last()?;

    let mut anchor_keys = Vec::new();
    let mut anchor_path = NodePath::root();
    for node in hit {
        if let Some(key) = node.node_key() {
            anchor_keys.push(key.clone());
            anchor_path = node.path().clone();
        }
    }

    let leaf_path = leaf.path().as_slice();
    let relative_path = NodePath::new(
        leaf_path
            .get(anchor_path.as_slice().len()..)
            .unwrap_or_default()
            .to_vec(),
    );

    Some(NodeRef::new(anchor_keys, relative_path))
}
