use super::fixture::{PARENT, embed};
use super::lifecycle::make_test_rect_node;
use super::outcome::plain_event_loop;
use crate::features::layout_engine::domain::{
    DisplayCommand, FloatingKind, PopupTarget, ShowFloating,
};
use crate::features::module_runtime::domain::render_pipeline::{EmbedReport, PipelineLayout};
use crate::features::module_runtime::domain::{FloatingLayouts, RenderOutcome};
use crate::shared::primitives::geometry::Scale;
use crate::shared::primitives::{LayoutSurface, MonitorId};

/// Regression test for Gap B: a module embedded inside an open popup that
/// itself shows a tooltip must dispatch that tooltip with the real floating
/// parent (the popup it's nested in), not a hardcoded `None` — the same
/// parent `dispatch_outcome_popups`/`dispatch_outcome_panels` already
/// compute via `surface_parent_for` for this module's own floating content.
#[test]
fn test_event_loop_tooltip_dispatch_carries_floating_parent_when_embedded() {
    let parent_id = PARENT;
    let mon = MonitorId::new("DP-1");

    // This module is itself embedded (via `ui.module()`) inside module
    // `parent_id`'s open popup — exactly the "module-in-popup" case.
    let (event_loop, wiring) = plain_event_loop();
    let mut event_loop = embed(event_loop, LayoutSurface::Popup);

    let outcome = RenderOutcome::new(
        PipelineLayout::new(
            make_test_rect_node(),
            FloatingLayouts::new(None, None, Some(make_test_rect_node())),
            EmbedReport::new(None, vec![]),
        ),
        None,
        Scale::new(1.0),
    );
    event_loop.dispatch_render_outcome(&mon, &outcome);

    let cmd = wiring.sent.display.try_recv();
    let Ok(DisplayCommand::ShowFloatingSurface(ShowFloating { kind, parent, .. })) = cmd else {
        panic!("Expected ShowFloatingSurface for the tooltip, got {cmd:?}");
    };
    assert_eq!(kind, FloatingKind::Tooltip);
    let popup = FloatingKind::Popup(PopupTarget::new(parent_id, mon));
    assert_eq!(parent, Some(popup));
}
