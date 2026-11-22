use super::fixture::MODULE;
use super::lifecycle::make_test_rect_node;
use super::outcome::plain_event_loop;
use crate::features::module_runtime::domain::render_pipeline::{EmbedReport, PipelineLayout};
use crate::features::module_runtime::domain::{FloatingLayouts, RenderOutcome, SizeChange};
use crate::features::module_runtime::ports::LayoutEvent;
use crate::shared::events::core::{
    PointerButton, PointerEvent, SurfaceKind, SurfaceLifecycleEvent,
};
use crate::shared::primitives::MonitorId;
use crate::shared::primitives::geometry::{Position, Scale, Size};

#[test]
fn test_dispatch_render_outcome_emits_layout_events() {
    let (mut event_loop, wiring) = plain_event_loop();
    let outcome = RenderOutcome::new(
        PipelineLayout::new(
            make_test_rect_node(),
            FloatingLayouts::default(),
            EmbedReport::new(
                Some(SizeChange::new(Size::new(0, 0), Size::new(50, 20))),
                vec![],
            ),
        ),
        None,
        Scale::new(1.0),
    );

    event_loop.dispatch_render_outcome(&MonitorId::new("DP-1"), &outcome);

    let received = wiring
        .sent
        .layout_events
        .try_recv()
        .expect("Should have sent ModuleSizeChanged");
    let LayoutEvent::ModuleSizeChanged {
        monitor_id,
        module_id,
        size,
        ..
    } = received
    else {
        panic!("Unexpected event");
    };
    assert_eq!(monitor_id.as_str(), "DP-1");
    assert_eq!(module_id, MODULE);
    assert_eq!(size, Size::new(50, 20));
}

#[test]
fn test_handle_pointer_event_no_tree_returns_false() {
    let (mut event_loop, _wiring) = plain_event_loop();
    let event = PointerEvent::Click {
        surface: SurfaceKind::Bar,
        button: PointerButton::Left,
        pos: Position::new(10, 10),
    };
    let changed = event_loop.handle_pointer_event(&MonitorId::new("DP-1"), &event);
    assert!(!changed);
}

#[test]
fn test_handle_lifecycle_event_popup_dismissed_returns_true() {
    let (mut event_loop, _wiring) = plain_event_loop();
    let changed = event_loop.handle_lifecycle_event(
        &MonitorId::new("DP-1"),
        SurfaceLifecycleEvent::PopupDismissed,
    );
    assert!(changed);
}
