use super::{FloatingContentCaches, layout_floating};
use crate::features::layout_engine::domain::{
    LayoutError, RenderNode, StyledNode, StyledPanel, StyledPopup, TextMeasurer,
};
use crate::features::layout_engine::ports::LayoutEnginePort;
use crate::features::module_runtime::test_support::MockCanvasFactory;
use crate::features::styling::domain::ComputedStyle;
use crate::features::vdom::domain::{
    AnchorDirection, ExclusiveZone, InteractionContext, KeyboardInteractivity, NodePath, NodeRef,
    PanelAnchor, PanelLayer, SurfaceSpace, TextContent,
};
use crate::shared::config::domain::MarginConfig;
use crate::shared::primitives::geometry::{Position, Rect, Scale, Size};
use crate::shared::primitives::{MonitorId, PopupOffset, SizeConstraint};
use std::collections::HashMap;

/// Records the content it was asked to lay out, and returns a node on the
/// same surface that keeps the content's own tooltip.
#[derive(Default)]
struct RecordingEngine {
    laid_out: Vec<StyledNode>,
}

impl LayoutEnginePort for RecordingEngine {
    fn calculate_layout_with_constraints(
        &mut self,
        node: StyledNode,
        _measurer: &mut dyn TextMeasurer,
        _start_pos: Position,
        _constraint: SizeConstraint,
    ) -> Result<RenderNode, LayoutError> {
        let laid_out = leaf(
            node.path().surface(),
            Rect::new(Position::new(0, 0), Size::new(5, 5)),
            node.tooltip().cloned(),
        );
        self.laid_out.push(node);
        Ok(laid_out)
    }
}

fn content(surface: SurfaceSpace, text: &str, tooltip: Option<StyledNode>) -> StyledNode {
    StyledNode::Text {
        path: NodePath::root_in(surface),
        node_key: None,
        text: TextContent::new(text.to_string()),
        style: ComputedStyle::default(),
        on_click: None,
        on_hover: None,
        tooltip: tooltip.map(Box::new),
        popup: None,
        panel: None,
    }
}

fn leaf(surface: SurfaceSpace, rect: Rect, tooltip: Option<StyledNode>) -> RenderNode {
    RenderNode::Rect {
        path: NodePath::root_in(surface),
        node_key: None,
        rect,
        style: ComputedStyle::default(),
        on_click: None,
        on_hover: None,
        tooltip: tooltip.map(Box::new),
        popup: None,
        panel: None,
    }
}

fn bar(children: Vec<RenderNode>) -> RenderNode {
    RenderNode::Flex {
        path: NodePath::root_in(SurfaceSpace::Bar),
        node_key: None,
        rect: Rect::new(Position::new(0, 0), Size::new(100, 30)),
        children,
        style: ComputedStyle::default(),
        on_click: None,
        on_hover: None,
        tooltip: None,
        popup: None,
        panel: None,
    }
}

struct Engines {
    popup: RecordingEngine,
    panel: RecordingEngine,
    tooltip: RecordingEngine,
}

fn run(
    bar: &RenderNode,
    interaction: Option<&InteractionContext>,
) -> (super::FloatingLayouts, Engines) {
    let mut engines = Engines {
        popup: RecordingEngine::default(),
        panel: RecordingEngine::default(),
        tooltip: RecordingEngine::default(),
    };
    let layouts = layout_floating(
        bar,
        interaction,
        &MonitorId::new("DP-1"),
        FloatingContentCaches {
            popup: &mut HashMap::new(),
            panel: &mut HashMap::new(),
            tooltip: &mut HashMap::new(),
        },
        &mut engines.popup,
        &mut engines.panel,
        &mut engines.tooltip,
        &mut MockCanvasFactory,
        Scale::new(1.0),
        None,
        &mut Vec::new(),
    );
    (layouts, engines)
}

#[test]
fn bar_with_nothing_open_lays_out_no_floating_content() {
    let (layouts, engines) = run(&bar(vec![]), None);

    assert!(layouts.popup().is_none());
    assert!(layouts.panel().is_none());
    assert!(layouts.tooltip().is_none());
    assert_eq!(engines.popup.laid_out, Vec::<StyledNode>::new());
    assert_eq!(engines.panel.laid_out, Vec::<StyledNode>::new());
    assert_eq!(engines.tooltip.laid_out, Vec::<StyledNode>::new());
}

#[test]
fn each_open_floating_kind_is_laid_out_by_its_own_engine() {
    let popup_tip = content(SurfaceSpace::Tooltip, "popup tip", None);
    let popup_content = content(SurfaceSpace::Popup, "popup", Some(popup_tip.clone()));
    let panel_content = content(SurfaceSpace::Panel, "panel", None);
    let button_rect = Rect::new(Position::new(10, 2), Size::new(30, 20));

    let mut button = leaf(SurfaceSpace::Bar, button_rect, None);
    if let RenderNode::Rect { popup, .. } = &mut button {
        *popup = Some(StyledPopup::new(
            Box::new(popup_content.clone()),
            AnchorDirection::Bottom,
            Some(PopupOffset::new(3, 4)),
        ));
    }
    let mut launcher = leaf(SurfaceSpace::Bar, button_rect, None);
    if let RenderNode::Rect { panel, .. } = &mut launcher {
        *panel = Some(StyledPanel::new(
            Box::new(panel_content.clone()),
            PanelLayer::Overlay,
            PanelAnchor::all(),
            MarginConfig::default(),
            ExclusiveZone::none(),
            KeyboardInteractivity::None,
        ));
    }
    // The pointer is over the popup's content, which carries a tooltip.
    let hovered = NodeRef::new(Vec::new(), NodePath::root_in(SurfaceSpace::Popup));
    let interaction = InteractionContext::new(Some(hovered), None, None, true);

    let (layouts, engines) = run(&bar(vec![button, launcher]), Some(&interaction));

    assert_eq!(engines.popup.laid_out, vec![popup_content]);
    assert_eq!(engines.panel.laid_out, vec![panel_content]);
    // The tooltip is found on the popup tree laid out in this same pass.
    assert_eq!(engines.tooltip.laid_out, vec![popup_tip]);

    let popup = layouts.popup().unwrap();
    assert_eq!(popup.anchor_rect(), button_rect);
    assert_eq!(popup.offset(), Some(PopupOffset::new(3, 4)));
    assert_eq!(popup.node().path().surface(), SurfaceSpace::Popup);
    assert_eq!(
        layouts.panel().unwrap().path().surface(),
        SurfaceSpace::Panel
    );
    assert_eq!(
        layouts.tooltip().unwrap().path().surface(),
        SurfaceSpace::Tooltip
    );
}
