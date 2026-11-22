#[cfg(test)]
mod tests {
    use crate::features::styling::domain::{ElementQuery, InheritedStyle};
    use crate::features::styling::ports::StyleResolverPort;
    use crate::features::vdom::domain::{InteractedNodes, InteractionContext, TextContent, VNode};
    use crate::shared::primitives::MonitorFocus;

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
            query: &ElementQuery<'_>,
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
        use crate::features::vdom::domain::{
            AnchorSegment, NodeKey, NodePath, NodeRef, SurfaceSpace,
        };

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
        let hovered = NodeRef::new(
            vec![AnchorSegment::new(
                NodePath::new(SurfaceSpace::Bar, vec![1]),
                NodeKey::new("b").unwrap(),
            )],
            NodePath::root_in(SurfaceSpace::Bar),
        );
        let interaction = InteractionContext::new(
            InteractedNodes::new(Some(hovered), None, None),
            MonitorFocus::Unfocused,
        );

        // Re-render with "a" removed: "b" is now at path [0].
        let root = VNode::new_flex(vec![item_b], None, None, None, None, None);
        let resolver = RecordingResolver::new();
        let _ = root.resolve_styles(&resolver, Some(&interaction));

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
        use crate::features::vdom::domain::{
            AnchorSegment, NodeKey, NodePath, NodeRef, SurfaceSpace,
        };

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
        let hovered = NodeRef::new(
            vec![AnchorSegment::new(
                NodePath::new(SurfaceSpace::Bar, vec![1]),
                NodeKey::new("b").unwrap(),
            )],
            NodePath::new(SurfaceSpace::Bar, vec![0]),
        );
        let interaction = InteractionContext::new(
            InteractedNodes::new(Some(hovered), None, None),
            MonitorFocus::Unfocused,
        );

        // Re-render with "a" removed: "b" is now at [0], its label at [0, 0].
        let root = VNode::new_flex(vec![item_b], None, None, None, None, None);
        let resolver = RecordingResolver::new();
        let _ = root.resolve_styles(&resolver, Some(&interaction));

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
        use crate::features::vdom::domain::{
            AnchorSegment, NodeKey, NodePath, NodeRef, SurfaceSpace,
        };

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
            vec![
                AnchorSegment::new(
                    NodePath::new(SurfaceSpace::Bar, vec![0]),
                    NodeKey::new("list-a").unwrap(),
                ),
                AnchorSegment::new(
                    NodePath::new(SurfaceSpace::Bar, vec![0]),
                    NodeKey::new("1").unwrap(),
                ),
            ],
            NodePath::root_in(SurfaceSpace::Bar),
        );
        let interaction = InteractionContext::new(
            InteractedNodes::new(Some(hovered), None, None),
            MonitorFocus::Unfocused,
        );

        let resolver = RecordingResolver::new();
        let _ = root.resolve_styles(&resolver, Some(&interaction));

        assert_eq!(
            resolver.pseudo_classes_of("list-a-item"),
            vec![crate::features::styling::domain::PseudoClass::Hover]
        );
        assert_eq!(resolver.pseudo_classes_of("list-b-item"), []);
    }

    /// Two keyed lists whose containers are *unkeyed* both reduce to the
    /// anchor chain `["1"]`, since unkeyed nodes are transparent to it. A
    /// hover captured in the second list must not land on the first.
    #[test]
    fn test_same_key_under_unkeyed_wrappers_does_not_cross_match() {
        use crate::features::styling::domain::ElementId;
        use crate::features::vdom::domain::{
            AnchorSegment, NodeKey, NodePath, NodeRef, SurfaceSpace,
        };

        let keyed_item = |id: &str| {
            VNode::new_text(
                TextContent::new(id.to_string()),
                None,
                Some(ElementId::new(id).unwrap()),
                None,
                None,
                None,
            )
            .with_key(NodeKey::new("1").unwrap())
        };
        let list_a = VNode::new_flex(vec![keyed_item("a-item")], None, None, None, None, None);
        let list_b = VNode::new_flex(vec![keyed_item("b-item")], None, None, None, None, None);
        let root = VNode::new_flex(vec![list_a, list_b], None, None, None, None, None);

        // What capture_node_ref produces for a hit on list B's item: the
        // unkeyed wrapper's position ([1]) is part of the anchor.
        let hovered = NodeRef::new(
            vec![AnchorSegment::new(
                NodePath::new(SurfaceSpace::Bar, vec![1, 0]),
                NodeKey::new("1").unwrap(),
            )],
            NodePath::root_in(SurfaceSpace::Bar),
        );
        let interaction = InteractionContext::new(
            InteractedNodes::new(Some(hovered), None, None),
            MonitorFocus::Unfocused,
        );

        let resolver = RecordingResolver::new();
        let _ = root.resolve_styles(&resolver, Some(&interaction));

        assert_eq!(resolver.pseudo_classes_of("a-item"), []);
        assert_eq!(
            resolver.pseudo_classes_of("b-item"),
            vec![crate::features::styling::domain::PseudoClass::Hover]
        );
    }

    fn panel_margin() -> crate::shared::config::domain::MarginConfig {
        use crate::shared::config::domain::{AxisMargins, MarginConfig, MarginOffset};
        MarginConfig::new(
            AxisMargins::new(MarginOffset::new(1), MarginOffset::new(2)),
            AxisMargins::new(MarginOffset::new(3), MarginOffset::new(4)),
        )
    }

    /// A bar button (`#button`) carrying a tooltip (`#tip`), a popup
    /// (`#pop`) and a panel (`#pan`), each a single text node.
    fn button_with_every_floating_kind() -> VNode {
        use crate::features::styling::domain::ElementId;
        use crate::features::vdom::domain::{
            AnchorDirection, ExclusiveZone, KeyboardInteractivity, PanelAnchor, PanelLayer,
            PanelSpec, PopupOffset, PopupSpec,
        };

        let text = |id: &str, tooltip: Option<Box<VNode>>| {
            VNode::new_text(
                TextContent::new(id.to_string()),
                None,
                Some(ElementId::new(id).unwrap()),
                None,
                None,
                tooltip,
            )
        };
        text("button", Some(Box::new(text("tip", None))))
            .with_popup(
                PopupSpec::new(Box::new(text("pop", None)))
                    .with_anchor(AnchorDirection::Top)
                    .with_offset(Some(PopupOffset::new(3, 4))),
            )
            .with_panel(
                PanelSpec::new(Box::new(text("pan", None)))
                    .with_layer(PanelLayer::Overlay)
                    .with_anchor(PanelAnchor::all())
                    .with_margin(panel_margin())
                    .with_exclusive_zone(ExclusiveZone::new(7))
                    .with_keyboard(KeyboardInteractivity::Exclusive),
            )
    }

    /// The button and each of its floating contents sit at the root path of
    /// their own surface. A hover captured on one surface must reach only
    /// the node on that surface, never the same-shaped path on another.
    #[test]
    fn test_hover_stays_on_the_surface_it_was_captured_on() {
        use crate::features::styling::domain::PseudoClass;
        use crate::features::vdom::domain::{NodePath, NodeRef, SurfaceSpace};

        let cases = [
            (SurfaceSpace::Bar, "button"),
            (SurfaceSpace::Tooltip, "tip"),
            (SurfaceSpace::Popup, "pop"),
            (SurfaceSpace::Panel, "pan"),
        ];
        for (surface, hovered_id) in cases {
            let hovered = NodeRef::new(Vec::new(), NodePath::root_in(surface));
            let interaction = InteractionContext::new(
                InteractedNodes::new(Some(hovered), None, None),
                MonitorFocus::Unfocused,
            );
            let resolver = RecordingResolver::new();
            let _ = button_with_every_floating_kind().resolve_styles(&resolver, Some(&interaction));

            for id in ["button", "tip", "pop", "pan"] {
                let expected = if id == hovered_id {
                    vec![PseudoClass::Hover]
                } else {
                    Vec::new()
                };
                assert_eq!(
                    resolver.pseudo_classes_of(id),
                    expected,
                    "hover on {surface:?}, element #{id}"
                );
            }
        }
    }

    /// Resolving styles must hand the layout engine the same popup and
    /// panel placement the script declared, on a tree rooted in that
    /// floating kind's own surface.
    #[test]
    fn test_floating_specs_keep_their_placement_through_style_resolution() {
        use crate::features::vdom::domain::{
            AnchorDirection, ExclusiveZone, KeyboardInteractivity, NodePath, PanelAnchor,
            PanelLayer, PopupOffset, SurfaceSpace,
        };

        let styled =
            button_with_every_floating_kind().resolve_styles(&RecordingResolver::new(), None);

        let tooltip = styled.tooltip().unwrap();
        assert_eq!(*tooltip.path(), NodePath::root_in(SurfaceSpace::Tooltip));

        let popup = styled.popup().unwrap();
        assert_eq!(
            *popup.content().path(),
            NodePath::root_in(SurfaceSpace::Popup)
        );
        assert_eq!(popup.anchor_direction(), AnchorDirection::Top);
        assert_eq!(popup.offset(), Some(PopupOffset::new(3, 4)));

        let panel = styled.panel().unwrap();
        assert_eq!(
            *panel.content().path(),
            NodePath::root_in(SurfaceSpace::Panel)
        );
        assert_eq!(panel.layer(), PanelLayer::Overlay);
        assert_eq!(panel.anchor(), PanelAnchor::all());
        assert_eq!(*panel.margin(), panel_margin());
        assert_eq!(panel.exclusive_zone(), ExclusiveZone::new(7));
        assert_eq!(panel.keyboard(), KeyboardInteractivity::Exclusive);
    }
}
