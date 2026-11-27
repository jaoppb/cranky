//! Pins how each `VNode` kind maps onto its `StyledNode` variant, with every attribute the node
//! carries (key, click and hover actions, tooltip, popup, panel) in its own slot.
#[cfg(test)]
mod tests {
    use crate::features::layout_engine::domain::{StyledNode, StyledPanel, StyledPopup};
    use crate::features::styling::domain::{
        ComputedStyle, ElementQuery, InheritedStyle, Orientation, ProgressValue,
    };
    use crate::features::styling::ports::StyleResolverPort;
    use crate::features::vdom::domain::{
        AnchorDirection, ClickHandlers, ElementSelectors, ImageContent, Interactions, ModuleParams,
        NodeKey, NodePath, PanelLayer, PanelSpec, PopupOffset, PopupSpec, SurfaceSpace,
        TextContent, UiAction, VNode,
    };
    use crate::shared::primitives::geometry::Size;
    use crate::shared::primitives::{
        BinaryData, ModuleInstanceId, ModuleKey, ModuleName, ModuleOptions,
    };

    /// A resolver that matches nothing, so every node keeps its kind's default style.
    struct PlainResolver;

    impl StyleResolverPort for PlainResolver {
        fn resolve_style(
            &self,
            _query: &ElementQuery<'_>,
            _inherited: &InheritedStyle,
        ) -> (ComputedStyle, InheritedStyle) {
            (ComputedStyle::default(), InheritedStyle::default())
        }
    }

    fn click() -> ClickHandlers {
        ClickHandlers::from_single(UiAction::Exec("click".into()))
    }

    fn hover() -> UiAction {
        UiAction::Exec("hover".into())
    }

    fn key() -> NodeKey {
        NodeKey::new("key").unwrap()
    }

    fn plain_text(label: &str) -> VNode {
        VNode::new_text(
            TextContent::new(label.to_string()),
            ElementSelectors::default(),
            Interactions::default(),
        )
    }

    /// The interactions every mapped node carries: a click, a hover and a `tip` tooltip.
    fn interactions() -> Interactions {
        Interactions::new(
            Some(click()),
            Some(hover()),
            Some(Box::new(plain_text("tip"))),
        )
    }

    /// A `pan` panel on the overlay layer.
    fn panel_spec() -> PanelSpec {
        PanelSpec::new(Box::new(plain_text("pan"))).with_layer(PanelLayer::Overlay)
    }

    /// `node` resolved with a key, a `pop` popup anchored to the top and a `pan` panel.
    fn resolve_decorated(node: VNode) -> StyledNode {
        let popup = PopupSpec::new(Box::new(plain_text("pop")))
            .with_anchor(AnchorDirection::Top)
            .with_offset(Some(PopupOffset::new(3, 4)));
        node.with_key(key())
            .with_popup(popup)
            .with_panel(panel_spec())
            .resolve_styles(&PlainResolver, None)
    }

    /// The styled node `plain_text(label)` resolves to at `path`.
    fn styled_text(label: &str, path: NodePath) -> StyledNode {
        StyledNode::Text {
            path,
            node_key: None,
            text: TextContent::new(label.to_string()),
            style: ComputedStyle::default(),
            on_click: None,
            on_hover: None,
            tooltip: None,
            popup: None,
            panel: None,
        }
    }

    fn styled_tooltip() -> Box<StyledNode> {
        let path = NodePath::root_in(SurfaceSpace::Tooltip);
        Box::new(styled_text("tip", path))
    }

    fn styled_popup() -> StyledPopup {
        let content = Box::new(styled_text("pop", NodePath::root_in(SurfaceSpace::Popup)));
        let offset = Some(PopupOffset::new(3, 4));
        StyledPopup::new(content, AnchorDirection::Top, offset)
    }

    fn styled_panel() -> StyledPanel {
        let content = Box::new(styled_text("pan", NodePath::root_in(SurfaceSpace::Panel)));
        StyledPanel::new(content, panel_spec().surface())
    }

    fn bar_root() -> NodePath {
        NodePath::root_in(SurfaceSpace::Bar)
    }

    /// The styled `child` text at the first child path of the bar root.
    fn styled_child() -> StyledNode {
        styled_text("child", NodePath::new(SurfaceSpace::Bar, vec![0]))
    }

    #[test]
    fn test_flex_maps_every_attribute_into_its_styled_slot() {
        let node = VNode::new_flex(
            vec![plain_text("child")],
            ElementSelectors::default(),
            interactions(),
        );
        let expected = StyledNode::Flex {
            path: bar_root(),
            node_key: Some(key()),
            children: vec![styled_child()],
            style: ComputedStyle::default_for_flex(),
            on_click: Some(click()),
            on_hover: Some(hover()),
            tooltip: Some(styled_tooltip()),
            popup: Some(styled_popup()),
            panel: Some(styled_panel()),
        };
        assert_eq!(resolve_decorated(node), expected);
    }

    #[test]
    fn test_grid_maps_every_attribute_into_its_styled_slot() {
        let node = VNode::new_grid(
            vec![plain_text("child")],
            ElementSelectors::default(),
            interactions(),
        );
        let expected = StyledNode::Grid {
            path: bar_root(),
            node_key: Some(key()),
            children: vec![styled_child()],
            style: ComputedStyle::default_for_grid(),
            on_click: Some(click()),
            on_hover: Some(hover()),
            tooltip: Some(styled_tooltip()),
            popup: Some(styled_popup()),
            panel: Some(styled_panel()),
        };
        assert_eq!(resolve_decorated(node), expected);
    }

    #[test]
    fn test_text_maps_every_attribute_into_its_styled_slot() {
        let text = TextContent::new("hello".to_string());
        let node = VNode::new_text(text.clone(), ElementSelectors::default(), interactions());
        let expected = StyledNode::Text {
            path: bar_root(),
            node_key: Some(key()),
            text,
            style: ComputedStyle::default(),
            on_click: Some(click()),
            on_hover: Some(hover()),
            tooltip: Some(styled_tooltip()),
            popup: Some(styled_popup()),
            panel: Some(styled_panel()),
        };
        assert_eq!(resolve_decorated(node), expected);
    }

    #[test]
    fn test_progress_maps_every_attribute_into_its_styled_slot() {
        let value = ProgressValue::new(0.3).unwrap();
        let bar = crate::features::vdom::domain::ProgressBar::new(value, Orientation::Vertical);
        let node = VNode::new_progress(bar, ElementSelectors::default(), interactions());
        let expected = StyledNode::Progress {
            path: bar_root(),
            node_key: Some(key()),
            value,
            orientation: Orientation::Vertical,
            style: ComputedStyle::default(),
            on_click: Some(click()),
            on_hover: Some(hover()),
            tooltip: Some(styled_tooltip()),
            popup: Some(styled_popup()),
            panel: Some(styled_panel()),
        };
        assert_eq!(resolve_decorated(node), expected);
    }

    #[test]
    fn test_rect_maps_every_attribute_into_its_styled_slot() {
        let node = VNode::new_rect(ElementSelectors::default(), interactions());
        let expected = StyledNode::Rect {
            path: bar_root(),
            node_key: Some(key()),
            style: ComputedStyle::default(),
            on_click: Some(click()),
            on_hover: Some(hover()),
            tooltip: Some(styled_tooltip()),
            popup: Some(styled_popup()),
            panel: Some(styled_panel()),
        };
        assert_eq!(resolve_decorated(node), expected);
    }

    #[test]
    fn test_image_maps_every_attribute_into_its_styled_slot() {
        let image = ImageContent::new(vec![1, 2, 3, 4], Size::new(1, 1));
        let tooltip = Some(Box::new(plain_text("tip")));
        let node = VNode::new_image(image, ElementSelectors::default(), tooltip);
        let expected = StyledNode::Image {
            path: bar_root(),
            node_key: Some(key()),
            data: BinaryData::new(vec![1, 2, 3, 4]),
            pixel_size: Size::new(1, 1),
            style: ComputedStyle::default(),
            tooltip: Some(styled_tooltip()),
            popup: Some(styled_popup()),
            panel: Some(styled_panel()),
        };
        assert_eq!(resolve_decorated(node), expected);
    }

    #[test]
    fn test_module_maps_every_attribute_into_its_styled_slot() {
        let (name, instance) = (ModuleName::new("clock"), ModuleInstanceId::new("left"));
        let params = ModuleParams::new(
            name.clone(),
            Some(instance.clone()),
            ModuleOptions::default(),
        );
        let node = VNode::new_module(params, ElementSelectors::default(), interactions());
        let expected = StyledNode::Module {
            path: bar_root(),
            node_key: Some(key()),
            key: ModuleKey::new(name, Some(instance)),
            options: ModuleOptions::default(),
            style: ComputedStyle::default(),
            on_click: Some(click()),
            on_hover: Some(hover()),
            tooltip: Some(styled_tooltip()),
            popup: Some(styled_popup()),
            panel: Some(styled_panel()),
        };
        assert_eq!(resolve_decorated(node), expected);
    }
}
