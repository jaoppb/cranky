use super::node_key::NodeKey;
use super::node_path::NodePath;
use super::node_ref::AnchorSegment;

/// A tree whose nodes can be located by walking a chain of `AnchorSegment`s.
///
/// See `NodeRef`: every step but the last keyed node is found by child
/// index, the keyed node itself by key. `VNode` (vdom) and `RenderNode`
/// (`layout_engine`) both shape their trees this way, so implementing this
/// once lets `resolve_segment` be written once and shared between them
/// instead of duplicated per tree type.
pub trait KeyedTree: Sized {
    /// This node's children, or an empty slice for a leaf.
    fn children(&self) -> &[Self];

    /// This node's reconciliation key, if it has one.
    fn node_key(&self) -> Option<&NodeKey>;
}

/// Follows one anchor segment from `start`.
///
/// Every step but the last walks through unkeyed nodes by position, then
/// the keyed node is picked out of that parent's children by key — its
/// captured index is ignored, since a reorder is exactly what changes it.
pub fn resolve_segment<'a, T: KeyedTree>(
    start: &'a T,
    start_path: NodePath,
    segment: &AnchorSegment,
) -> Option<(&'a T, NodePath)> {
    let Some((_, through)) = segment.path().as_slice().split_last() else {
        return (start.node_key() == Some(segment.key())).then_some((start, start_path));
    };

    let mut parent = start;
    let mut parent_path = start_path;
    for &idx in through {
        parent = parent.children().get(idx)?;
        parent_path = parent_path.child(idx);
    }

    parent
        .children()
        .iter()
        .enumerate()
        .find(|(_, child)| child.node_key() == Some(segment.key()))
        .map(|(idx, child)| (child, parent_path.child(idx)))
}
