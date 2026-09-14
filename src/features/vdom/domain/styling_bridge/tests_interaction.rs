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

        fn pseudo_classes_of(&self, id: &str) -> Vec<crate::features::styling::domain::PseudoClass> {
            self.seen.lock().unwrap().get(id).cloned().unwrap_or_default()
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

        // Captured as if the pointer hit "b" while "a" (since removed)
        // still came before it, putting "b" at path [1].
        let hovered = NodeRef::new(NodePath::new(vec![1]), Some(NodeKey::new("b").unwrap()));
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
}
