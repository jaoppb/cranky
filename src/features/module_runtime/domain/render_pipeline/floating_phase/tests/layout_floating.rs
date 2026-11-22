use super::super::{
    FloatingEngine, FloatingEngines, FloatingLayouts, FloatingPass, layout_floating,
};
use super::{RecordingEngine, Spec, content, panel_on_every_edge, tooltip_content};
use crate::features::layout_engine::domain::{RenderNode, StyledNode, StyledPanel, StyledPopup};
use crate::features::module_runtime::test_support::MockCanvasFactory;
use crate::features::vdom::domain::{
    AnchorDirection, InteractedNodes, InteractionContext, NodePath, NodeRef, PanelLayer,
    SurfaceSpace,
};
use crate::shared::primitives::geometry::{Position, Rect, Scale, Size};
use crate::shared::primitives::{MonitorFocus, MonitorId, PopupOffset};
use std::collections::HashMap;

#[derive(Default)]
struct Engines {
    popup: RecordingEngine,
    panel: RecordingEngine,
    tooltip: RecordingEngine,
}

fn run(bar: &RenderNode, interaction: Option<&InteractionContext>) -> (FloatingLayouts, Engines) {
    let mut engines = Engines::default();
    let layouts = layout_floating(
        bar,
        FloatingEngines {
            popup: FloatingEngine {
                engine: &mut engines.popup,
                cache: &mut HashMap::new(),
            },
            panel: FloatingEngine {
                engine: &mut engines.panel,
                cache: &mut HashMap::new(),
            },
            tooltip: FloatingEngine {
                engine: &mut engines.tooltip,
                cache: &mut HashMap::new(),
            },
        },
        &mut FloatingPass {
            monitor_id: &MonitorId::new("DP-1"),
            canvas_factory: &mut MockCanvasFactory,
            scale: Scale::new(1.0),
            current_child_sizes: None,
            interaction,
        },
    );
    (layouts, engines)
}

#[test]
fn bar_with_nothing_open_lays_out_no_floating_content() {
    let bar = Spec::at_path(SurfaceSpace::Bar, &[]).flex(vec![]);
    let (layouts, engines) = run(&bar, None);

    assert!(layouts.popup().is_none());
    assert!(layouts.panel().is_none());
    assert!(layouts.tooltip().is_none());
    assert_eq!(engines.popup.laid_out, Vec::<StyledNode>::new());
    assert_eq!(engines.panel.laid_out, Vec::<StyledNode>::new());
    assert_eq!(engines.tooltip.laid_out, Vec::<StyledNode>::new());
}

#[test]
fn each_open_floating_kind_is_laid_out_by_its_own_engine() {
    let popup_tip = tooltip_content("popup tip");
    let popup_content = content(SurfaceSpace::Popup, "popup", Some(popup_tip.clone()));
    let panel_content = content(SurfaceSpace::Panel, "panel", None);
    let button_rect = Rect::new(Position::new(10, 2), Size::new(30, 20));

    let button = Spec {
        rect: button_rect,
        popup: Some(StyledPopup::new(
            Box::new(popup_content.clone()),
            AnchorDirection::Bottom,
            Some(PopupOffset::new(3, 4)),
        )),
        ..Spec::at_path(SurfaceSpace::Bar, &[0])
    }
    .leaf();
    let launcher = Spec {
        rect: Rect::new(Position::new(60, 2), Size::new(15, 20)),
        panel: Some(StyledPanel::new(
            Box::new(panel_content.clone()),
            panel_on_every_edge(PanelLayer::Overlay),
        )),
        ..Spec::at_path(SurfaceSpace::Bar, &[1])
    }
    .leaf();
    let bar = Spec::at_path(SurfaceSpace::Bar, &[]).flex(vec![button, launcher]);
    // The pointer is over the popup's content, which carries a tooltip.
    let hovered = NodeRef::new(Vec::new(), NodePath::root_in(SurfaceSpace::Popup));
    let interaction = InteractionContext::new(
        InteractedNodes::new(Some(hovered), None, None),
        MonitorFocus::Focused,
    );

    let (layouts, engines) = run(&bar, Some(&interaction));

    assert_eq!(engines.popup.laid_out, vec![popup_content]);
    assert_eq!(engines.panel.laid_out, vec![panel_content]);
    // The tooltip is found on the popup tree laid out in this same pass.
    assert_eq!(engines.tooltip.laid_out, vec![popup_tip]);

    let popup = layouts.popup().unwrap();
    // Anchored to the node that owns the popup, not to the panel's owner.
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
