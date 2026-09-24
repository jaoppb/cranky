use crate::features::layout_engine::domain::RenderNode;
use crate::features::vdom::domain::{AnchorSegment, NodePath, NodeRef};

/// Captures a `NodeRef` for the deepest node in a hit-test chain (root to
/// leaf, as returned by `RenderNode::hit_test`): one anchor segment per
/// keyed node on the chain, each holding the path from the previous keyed
/// node (or root), then the path from the last keyed node down to the leaf.
///
/// Using only the leaf (e.g. an unkeyed text label inside a keyed list
/// item) would strand the interaction on that item's key entirely, and
/// anchoring at the keyed ancestor alone would drop the leaf's own
/// identity. See `NodeRef` for why unkeyed levels are kept positionally
/// rather than skipped.
#[must_use]
pub(super) fn capture_node_ref(hit: &[&RenderNode]) -> Option<NodeRef> {
    let leaf = hit.last()?;

    let mut anchor = Vec::new();
    let mut anchored_len = 0;
    for node in hit {
        if let Some(key) = node.node_key() {
            let path = node.path().as_slice();
            anchor.push(AnchorSegment::new(
                suffix_from(path, anchored_len),
                key.clone(),
            ));
            anchored_len = path.len();
        }
    }

    let relative_path = suffix_from(leaf.path().as_slice(), anchored_len);
    Some(NodeRef::new(anchor, relative_path))
}

fn suffix_from(path: &[usize], start: usize) -> NodePath {
    NodePath::new(path.get(start..).unwrap_or_default().to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::styling::domain::ComputedStyle;
    use crate::features::vdom::domain::NodeKey;
    use crate::shared::primitives::geometry::{Position, Rect, Size};

    fn node(path: &[usize], key: Option<&str>, x: i32, children: Vec<RenderNode>) -> RenderNode {
        RenderNode::Flex {
            path: NodePath::new(path.to_vec()),
            node_key: key.map(|k| NodeKey::new(k).unwrap()),
            rect: Rect::new(Position::new(x, 0), Size::new(100, 10)),
            children,
            style: ComputedStyle::default(),
            on_click: None,
            on_hover: None,
            tooltip: None,
            popup: None,
            panel: None,
        }
    }

    #[test]
    fn test_capture_keeps_unkeyed_wrapper_position_in_anchor() {
        // root -> [list A (unkeyed) -> item "1", list B (unkeyed) -> item "1"]
        let list_a = node(&[0], None, 0, vec![node(&[0, 0], Some("1"), 0, vec![])]);
        let list_b = node(&[1], None, 100, vec![node(&[1, 0], Some("1"), 100, vec![])]);
        let mut root = node(&[], None, 0, vec![list_a, list_b]);
        if let RenderNode::Flex { rect, .. } = &mut root {
            *rect = Rect::new(Position::new(0, 0), Size::new(200, 10));
        }

        let hit = root.hit_test(Position::new(150, 5));
        let captured = capture_node_ref(&hit).unwrap();

        assert_eq!(
            captured.anchor(),
            &[AnchorSegment::new(
                NodePath::new(vec![1, 0]),
                NodeKey::new("1").unwrap()
            )]
        );
        assert!(captured.relative_path().is_root());
    }

    #[test]
    fn test_capture_splits_nested_keyed_levels_and_leaf_path() {
        // root -> outer "row" -> unkeyed wrapper -> item "b" -> unkeyed label
        let label = node(&[0, 0, 0, 0], None, 0, vec![]);
        let item = node(&[0, 0, 0], Some("b"), 0, vec![label]);
        let wrapper = node(&[0, 0], None, 0, vec![item]);
        let row = node(&[0], Some("row"), 0, vec![wrapper]);
        let root = node(&[], None, 0, vec![row]);

        let hit = root.hit_test(Position::new(5, 5));
        let captured = capture_node_ref(&hit).unwrap();

        assert_eq!(
            captured.anchor(),
            &[
                AnchorSegment::new(NodePath::new(vec![0]), NodeKey::new("row").unwrap()),
                AnchorSegment::new(NodePath::new(vec![0, 0]), NodeKey::new("b").unwrap()),
            ]
        );
        assert_eq!(captured.relative_path().as_slice(), &[0]);
    }

    #[test]
    fn test_capture_without_keys_uses_absolute_leaf_path() {
        let root = node(&[], None, 0, vec![node(&[0], None, 0, vec![])]);

        let hit = root.hit_test(Position::new(5, 5));
        let captured = capture_node_ref(&hit).unwrap();

        assert!(captured.anchor().is_empty());
        assert_eq!(captured.relative_path().as_slice(), &[0]);
    }
}
