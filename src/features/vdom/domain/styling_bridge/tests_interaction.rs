#[cfg(test)]
mod tests {
    use crate::features::styling::domain::{ElementId, ElementQuery, InheritedStyle, PseudoClass};
    use crate::features::styling::ports::StyleResolverPort;
    use crate::features::vdom::domain::{
        AnchorSegment, ElementSelectors, InteractedNodes, InteractionContext, Interactions,
        NodeKey, NodePath, NodeRef, SurfaceSpace, TextContent, VNode,
    };
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
            let resolved = query
                .id()
                .map(|id| (id.as_str().to_string(), query.pseudo_classes().to_vec()));
            self.seen.lock().unwrap().extend(resolved);
            (
                crate::features::styling::domain::ComputedStyle::default(),
                InheritedStyle::default(),
            )
        }
    }

    /// A text node `#id` showing `content`.
    fn id_text(id: &str, content: &str) -> VNode {
        VNode::new_text(
            TextContent::new(content.to_string()),
            ElementSelectors::new(None, Some(ElementId::new(id).unwrap())),
            Interactions::default(),
        )
    }

    /// An unstyled flex container around `children`.
    fn flex(children: Vec<VNode>) -> VNode {
        VNode::new_flex(
            children,
            ElementSelectors::default(),
            Interactions::default(),
        )
    }

    fn key(name: &str) -> NodeKey {
        NodeKey::new(name).unwrap()
    }

    /// An anchor on the node keyed `name`, captured at bar path `indices`.
    fn segment(indices: Vec<usize>, name: &str) -> AnchorSegment {
        AnchorSegment::new(NodePath::new(SurfaceSpace::Bar, indices), key(name))
    }

    /// The pseudo-classes each element of `root` was resolved with while `hovered` is hovered on
    /// an unfocused monitor.
    fn resolve_hovering(root: &VNode, hovered: NodeRef) -> RecordingResolver {
        let interaction = InteractionContext::new(
            InteractedNodes::new(Some(hovered), None, None),
            MonitorFocus::Unfocused,
        );
        let resolver = RecordingResolver::new();
        let _ = root.resolve_styles(&resolver, Some(&interaction));
        resolver
    }

    fn hover() -> Vec<PseudoClass> {
        vec![PseudoClass::Hover]
    }

    /// A keyed node that survives its earlier sibling being removed (so its
    /// path shifts) must keep its `:hover` — matched by `NodeKey`, not by
    /// the stale path captured when the pointer first hit it. This is the
    /// styling side of the same fix `taffy`'s keyed reconciler tests cover
    /// for layout.
    #[test]
    fn test_hover_survives_reorder_when_matched_by_key() {
        let item_b = id_text("item-b", "b").with_key(key("b"));

        // Captured as if the pointer hit "b" itself while "a" (since
        // removed) still came before it, putting "b" at path [1] at capture
        // time. The anchor is "b" itself, so the relative path is root.
        let hovered = NodeRef::new(
            vec![segment(vec![1], "b")],
            NodePath::root_in(SurfaceSpace::Bar),
        );

        // Re-render with "a" removed: "b" is now at path [0].
        let resolver = resolve_hovering(&flex(vec![item_b]), hovered);

        assert_eq!(resolver.pseudo_classes_of("item-b"), hover());
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
        let label = id_text("label-b", "B");
        let item_b = VNode::new_flex(
            vec![label],
            ElementSelectors::new(None, Some(ElementId::new("item-b").unwrap())),
            Interactions::default(),
        )
        .with_key(key("b"));

        // Captured as if the pointer hit the unkeyed label while "a" (since
        // removed) still came before "b", putting the label at path [1, 0]
        // and its keyed container ("b") at [1]. The anchor is "b"; the
        // relative path from that anchor down to the label is [0].
        let hovered = NodeRef::new(
            vec![segment(vec![1], "b")],
            NodePath::new(SurfaceSpace::Bar, vec![0]),
        );

        // Re-render with "a" removed: "b" is now at [0], its label at [0, 0].
        let resolver = resolve_hovering(&flex(vec![item_b]), hovered);

        let item_and_label = ["item-b", "label-b"].map(|id| resolver.pseudo_classes_of(id));
        assert_eq!(item_and_label, [hover(), hover()]);
    }

    /// Two different lists reusing the same bare key ("1") must not
    /// cross-match — the anchor is the full key chain from root, not just
    /// the leaf key.
    #[test]
    fn test_same_key_in_different_lists_does_not_cross_match() {
        let list_a =
            flex(vec![id_text("list-a-item", "a1").with_key(key("1"))]).with_key(key("list-a"));
        let list_b =
            flex(vec![id_text("list-b-item", "b1").with_key(key("1"))]).with_key(key("list-b"));
        let root = flex(vec![list_a, list_b]);

        // Anchor chain ["list-a", "1"] must only ever resolve inside list-a.
        let hovered = NodeRef::new(
            vec![segment(vec![0], "list-a"), segment(vec![0], "1")],
            NodePath::root_in(SurfaceSpace::Bar),
        );

        let resolver = resolve_hovering(&root, hovered);

        let items = ["list-a-item", "list-b-item"].map(|id| resolver.pseudo_classes_of(id));
        assert_eq!(items, [hover(), Vec::new()]);
    }

    /// Two keyed lists whose containers are *unkeyed* both reduce to the
    /// anchor chain `["1"]`, since unkeyed nodes are transparent to it. A
    /// hover captured in the second list must not land on the first.
    #[test]
    fn test_same_key_under_unkeyed_wrappers_does_not_cross_match() {
        let keyed_item = |id: &str| id_text(id, id).with_key(key("1"));
        let list_a = flex(vec![keyed_item("a-item")]);
        let list_b = flex(vec![keyed_item("b-item")]);
        let root = flex(vec![list_a, list_b]);

        // What capture_node_ref produces for a hit on list B's item: the
        // unkeyed wrapper's position ([1]) is part of the anchor.
        let hovered = NodeRef::new(
            vec![segment(vec![1, 0], "1")],
            NodePath::root_in(SurfaceSpace::Bar),
        );

        let resolver = resolve_hovering(&root, hovered);

        let items = ["a-item", "b-item"].map(|id| resolver.pseudo_classes_of(id));
        assert_eq!(items, [Vec::new(), hover()]);
    }

    fn panel_margin() -> crate::shared::config::domain::MarginConfig {
        use crate::shared::config::domain::{AxisMargins, MarginConfig, MarginOffset};
        MarginConfig::new(
            AxisMargins::new(MarginOffset::new(1), MarginOffset::new(2)),
            AxisMargins::new(MarginOffset::new(3), MarginOffset::new(4)),
        )
    }

    /// A panel around `#pan`, with every placement field away from its default.
    fn placed_panel() -> crate::features::vdom::domain::PanelSpec {
        use crate::features::vdom::domain::{
            ExclusiveZone, KeyboardInteractivity, PanelAnchor, PanelLayer, PanelSpec,
        };
        PanelSpec::new(Box::new(id_text("pan", "pan")))
            .with_layer(PanelLayer::Overlay)
            .with_anchor(PanelAnchor::all())
            .with_margin(panel_margin())
            .with_exclusive_zone(ExclusiveZone::new(7))
            .with_keyboard(KeyboardInteractivity::Exclusive)
    }

    /// A bar button (`#button`) carrying a tooltip (`#tip`), a popup
    /// (`#pop`) and a panel (`#pan`), each a single text node.
    fn button_with_every_floating_kind() -> VNode {
        use crate::features::vdom::domain::{AnchorDirection, PopupOffset, PopupSpec};

        let tooltip = Some(Box::new(id_text("tip", "tip")));
        VNode::new_text(
            TextContent::new("button".to_string()),
            ElementSelectors::new(None, Some(ElementId::new("button").unwrap())),
            Interactions::new(None, None, tooltip),
        )
        .with_popup(
            PopupSpec::new(Box::new(id_text("pop", "pop")))
                .with_anchor(AnchorDirection::Top)
                .with_offset(Some(PopupOffset::new(3, 4))),
        )
        .with_panel(placed_panel())
    }

    /// Asserts that a hover captured at the root of `surface` marks only `#hovered_id`.
    fn assert_hover_reaches_only(surface: SurfaceSpace, hovered_id: &str) {
        let hovered = NodeRef::new(Vec::new(), NodePath::root_in(surface));
        let resolver = resolve_hovering(&button_with_every_floating_kind(), hovered);

        for id in ["button", "tip", "pop", "pan"] {
            let expected: Vec<_> = (id == hovered_id)
                .then_some(PseudoClass::Hover)
                .into_iter()
                .collect();
            assert_eq!(
                resolver.pseudo_classes_of(id),
                expected,
                "hover on {surface:?}, element #{id}"
            );
        }
    }

    /// The button and each of its floating contents sit at the root path of
    /// their own surface. A hover captured on one surface must reach only
    /// the node on that surface, never the same-shaped path on another.
    #[test]
    fn test_hover_stays_on_the_surface_it_was_captured_on() {
        let cases = [
            (SurfaceSpace::Bar, "button"),
            (SurfaceSpace::Tooltip, "tip"),
            (SurfaceSpace::Popup, "pop"),
            (SurfaceSpace::Panel, "pan"),
        ];
        for (surface, hovered_id) in cases {
            assert_hover_reaches_only(surface, hovered_id);
        }
    }

    /// Resolving styles must hand the layout engine the same popup and
    /// panel placement the script declared, on a tree rooted in that
    /// floating kind's own surface. The panel half is
    /// `test_panel_keeps_its_placement_through_style_resolution`.
    #[test]
    fn test_floating_specs_keep_their_placement_through_style_resolution() {
        use crate::features::vdom::domain::{AnchorDirection, PopupOffset};

        let styled =
            button_with_every_floating_kind().resolve_styles(&RecordingResolver::new(), None);

        let tooltip = styled.tooltip().unwrap();
        assert_eq!(*tooltip.path(), NodePath::root_in(SurfaceSpace::Tooltip));

        let popup = styled.popup().unwrap();
        let popup_root = NodePath::root_in(SurfaceSpace::Popup);
        let placement = (popup.anchor_direction(), popup.offset());
        let expected = (AnchorDirection::Top, Some(PopupOffset::new(3, 4)));
        assert_eq!((popup.content().path(), placement), (&popup_root, expected));
    }

    /// The panel's content is rooted in the panel surface and keeps every placement field the
    /// script declared.
    #[test]
    fn test_panel_keeps_its_placement_through_style_resolution() {
        use crate::features::vdom::domain::{
            ExclusiveZone, KeyboardInteractivity, PanelAnchor, PanelLayer,
        };

        let styled =
            button_with_every_floating_kind().resolve_styles(&RecordingResolver::new(), None);

        let panel = styled.panel().unwrap();
        let panel_root = NodePath::root_in(SurfaceSpace::Panel);
        assert_eq!(panel.content().path(), &panel_root);
        let layering = (panel.layer(), panel.anchor(), *panel.margin());
        let expected = (PanelLayer::Overlay, PanelAnchor::all(), panel_margin());
        assert_eq!(layering, expected);
        let interactivity = (panel.exclusive_zone(), panel.keyboard());
        let expected = (ExclusiveZone::new(7), KeyboardInteractivity::Exclusive);
        assert_eq!(interactivity, expected);
    }
}
