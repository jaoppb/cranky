#[cfg(test)]
mod tests {
    use crate::features::styling::domain::{ElementQuery, InheritedStyle};
    use crate::features::styling::ports::StyleResolverPort;
    use crate::features::vdom::domain::{InteractionContext, TextContent, VNode};

    /// Records the pseudo-classes each element id was resolved with, so a
    /// test can check `:hover`/`:active`/`:focus` landed on the right node
    /// without needing lightningcss's real selector matching.
    struct RecordingResolver {
        seen: std::sync::Mutex<
            std::collections::HashMap<String, Vec<crate::features::styling::domain::PseudoClass>>,
        >,
    }

    impl RecordingResolver {
        fn new() -> Self {
            Self {
                seen: std::sync::Mutex::new(std::collections::HashMap::new()),
            }
        }

        fn pseudo_classes_of(
            &self,
            id: &str,
        ) -> Vec<crate::features::styling::domain::PseudoClass> {
            self.seen
                .lock()
                .unwrap()
                .get(id)
                .cloned()
                .unwrap_or_default()
        }
    }

    impl StyleResolverPort for RecordingResolver {
        fn resolve_style(
            &self,
            query: &ElementQuery,
            _inherited: &InheritedStyle,
        ) -> (
            crate::features::styling::domain::ComputedStyle,
            InheritedStyle,
        ) {
            if let Some(id) = query.id() {
                self.seen
                    .lock()
                    .unwrap()
                    .insert(id.as_str().to_string(), query.pseudo_classes().to_vec());
            }
            (
                crate::features::styling::domain::ComputedStyle::default(),
                InheritedStyle::default(),
            )
        }
    }

    /// A keyed node that survives its earlier sibling being removed (so its
    /// path shifts) must keep its `:hover` — matched by `NodeKey`, not by
    /// the stale path captured when the pointer first hit it. This is the
    /// styling side of the same fix `taffy`'s keyed reconciler tests cover
    /// for layout.
    #[test]
    fn test_hover_survives_reorder_when_matched_by_key() {
        use crate::features::styling::domain::ElementId;
        use crate::features::vdom::domain::{NodeKey, NodePath, NodeRef};

        let item_b = VNode::new_text(
            TextContent::new("b".to_string()),
            None,
            Some(ElementId::new("item-b").unwrap()),
            None,
            None,
            None,
        )
        .with_key(NodeKey::new("b").unwrap());

        // Captured as if the pointer hit "b" itself while "a" (since
        // removed) still came before it, putting "b" at path [1] at capture
        // time. The anchor is "b" itself, so the relative path is root.
        let hovered = NodeRef::new(vec![NodeKey::new("b").unwrap()], NodePath::root());
        let interaction = InteractionContext::new(Some(hovered), None, None, false);

        // Re-render with "a" removed: "b" is now at path [0].
        let root = VNode::new_flex(vec![item_b], None, None, None, None, None);
        let resolver = RecordingResolver::new();
        let _ = root.resolve_styles(&resolver, Some(&interaction), None);

        assert_eq!(
            resolver.pseudo_classes_of("item-b"),
            vec![crate::features::styling::domain::PseudoClass::Hover]
        );
    }

    /// The exact shape `workspace.lua`/`systray.lua` use: a keyed container
    /// (`.item`) whose actual child is an unkeyed leaf (`.label`). The hit
    /// lands on the unkeyed leaf, so the captured ref's anchor is the
    /// keyed container, one level up. After an earlier sibling is removed
    /// and the container's absolute path shifts, both the keyed container
    /// AND its unkeyed child must still resolve to `:hover` — the container
    /// via ancestor bubbling from the reconstructed target path, the leaf
    /// via an exact match on that same target path.
    #[test]
    fn test_hover_reaches_both_keyed_container_and_unkeyed_child_after_reorder() {
        use crate::features::styling::domain::ElementId;
        use crate::features::vdom::domain::{NodeKey, NodePath, NodeRef};

        let label = VNode::new_text(
            TextContent::new("B".to_string()),
            None,
            Some(ElementId::new("label-b").unwrap()),
            None,
            None,
            None,
        );
        let item_b = VNode::new_flex(
            vec![label],
            None,
            Some(ElementId::new("item-b").unwrap()),
            None,
            None,
            None,
        )
        .with_key(NodeKey::new("b").unwrap());

        // Captured as if the pointer hit the unkeyed label while "a" (since
        // removed) still came before "b", putting the label at path [1, 0]
        // and its keyed container ("b") at [1]. The anchor is "b"; the
        // relative path from that anchor down to the label is [0].
        let hovered = NodeRef::new(vec![NodeKey::new("b").unwrap()], NodePath::new(vec![0]));
        let interaction = InteractionContext::new(Some(hovered), None, None, false);

        // Re-render with "a" removed: "b" is now at [0], its label at [0, 0].
        let root = VNode::new_flex(vec![item_b], None, None, None, None, None);
        let resolver = RecordingResolver::new();
        let _ = root.resolve_styles(&resolver, Some(&interaction), None);

        assert_eq!(
            resolver.pseudo_classes_of("item-b"),
            vec![crate::features::styling::domain::PseudoClass::Hover]
        );
        assert_eq!(
            resolver.pseudo_classes_of("label-b"),
            vec![crate::features::styling::domain::PseudoClass::Hover]
        );
    }

    /// Two different lists reusing the same bare key ("1") must not
    /// cross-match — the anchor is the full key chain from root, not just
    /// the leaf key.
    #[test]
    fn test_same_key_in_different_lists_does_not_cross_match() {
        use crate::features::styling::domain::ElementId;
        use crate::features::vdom::domain::{NodeKey, NodePath, NodeRef};

        let list_a_item = VNode::new_text(
            TextContent::new("a1".to_string()),
            None,
            Some(ElementId::new("list-a-item").unwrap()),
            None,
            None,
            None,
        )
        .with_key(NodeKey::new("1").unwrap());
        let list_a = VNode::new_flex(vec![list_a_item], None, None, None, None, None)
            .with_key(NodeKey::new("list-a").unwrap());

        let list_b_item = VNode::new_text(
            TextContent::new("b1".to_string()),
            None,
            Some(ElementId::new("list-b-item").unwrap()),
            None,
            None,
            None,
        )
        .with_key(NodeKey::new("1").unwrap());
        let list_b = VNode::new_flex(vec![list_b_item], None, None, None, None, None)
            .with_key(NodeKey::new("list-b").unwrap());

        let root = VNode::new_flex(vec![list_a, list_b], None, None, None, None, None);

        // Anchor chain ["list-a", "1"] must only ever resolve inside list-a.
        let hovered = NodeRef::new(
            vec![NodeKey::new("list-a").unwrap(), NodeKey::new("1").unwrap()],
            NodePath::root(),
        );
        let interaction = InteractionContext::new(Some(hovered), None, None, false);

        let resolver = RecordingResolver::new();
        let _ = root.resolve_styles(&resolver, Some(&interaction), None);

        assert_eq!(
            resolver.pseudo_classes_of("list-a-item"),
            vec![crate::features::styling::domain::PseudoClass::Hover]
        );
        assert!(resolver.pseudo_classes_of("list-b-item").is_empty());
    }
}
