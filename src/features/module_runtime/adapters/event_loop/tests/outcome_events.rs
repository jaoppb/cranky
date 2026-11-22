//! What dispatching one render outcome sends to the embedder and the display: layout events,
//! and floating content at the scale it was rendered at.
use super::fixture::MODULE;
use super::lifecycle::make_test_rect_node;
use super::outcome::plain_event_loop;
use crate::features::layout_engine::domain::{DisplayCommand, ShowFloating};
use crate::features::module_runtime::domain::render_pipeline::{EmbedReport, PipelineLayout};
use crate::features::module_runtime::domain::{
    FloatingLayouts, PopupRenderLayout, RenderOutcome, SizeChange,
};
use crate::features::module_runtime::ports::LayoutEvent;
use crate::shared::primitives::geometry::{Position, Rect, Scale, Size};
use crate::shared::primitives::{
    ChildBounds, ChildModuleLayout, LayoutSurface, ModuleKey, ModuleName, MonitorId, SizeConstraint,
};

fn clock_layout() -> ChildModuleLayout {
    ChildModuleLayout::new(
        ModuleKey::from_name(ModuleName::new("clock")),
        ChildBounds::new(
            Rect::new(Position::new(3, 1), Size::new(40, 18)),
            SizeConstraint::new(Some(40), None),
        ),
        LayoutSurface::Bar,
    )
}

/// An outcome at `scale` with `floating` open, reporting `report`.
fn outcome(floating: FloatingLayouts, report: EmbedReport, scale: f32) -> RenderOutcome {
    RenderOutcome::new(
        PipelineLayout::new(make_test_rect_node(), floating, report),
        None,
        Scale::new(scale),
    )
}

/// Child layouts go out while there are any, once more empty when the last one goes, then not
/// at all; a size change goes out with the settled width.
#[test]
fn layout_events_carry_the_module_monitor_children_and_settled_size() {
    let monitor = MonitorId::new("HDMI-A-1");
    let (mut event_loop, wiring) = plain_event_loop();
    let grown = SizeChange::new(Size::new(0, 0), Size::new(64, 22)).with_settled_width(Some(280));
    let report = EmbedReport::new(Some(grown), vec![clock_layout()]);
    let with_child = outcome(FloatingLayouts::default(), report, 1.0);
    event_loop.dispatch_render_outcome(&monitor, &with_child);
    let empty = outcome(
        FloatingLayouts::default(),
        EmbedReport::new(None, vec![]),
        1.0,
    );
    event_loop.dispatch_render_outcome(&monitor, &empty);
    event_loop.dispatch_render_outcome(&monitor, &empty);
    event_loop.dispatch_render_outcome(&MonitorId::new("DP-1"), &empty);

    let children = |layouts| LayoutEvent::ContainerLayoutsCalculated {
        parent_id: MODULE,
        monitor_id: monitor.clone(),
        layouts,
    };
    let size = LayoutEvent::ModuleSizeChanged {
        monitor_id: monitor.clone(),
        module_id: MODULE,
        size: Size::new(64, 22),
        settled_width: Some(280),
    };
    let sent: Vec<_> = wiring.sent.layout_events.try_iter().collect();
    assert_eq!(
        sent,
        vec![children(vec![clock_layout()]), size, children(vec![])]
    );
}

/// Floating content is painted at the scale the outcome was dispatched with.
#[test]
fn floating_content_is_painted_at_the_render_scale() {
    let (mut event_loop, wiring) = plain_event_loop();
    let anchor = Rect::new(Position::new(8, 0), Size::new(16, 20));
    let popup = PopupRenderLayout::new(anchor, make_test_rect_node(), None);
    let floating = FloatingLayouts::new(Some(popup), None, None);

    let with_popup = outcome(floating, EmbedReport::new(None, vec![]), 2.0);
    event_loop.dispatch_render_outcome(&MonitorId::new("DP-1"), &with_popup);

    let Ok(DisplayCommand::ShowFloatingSurface(ShowFloating {
        buffer,
        logical_size,
        anchor_rect,
        ..
    })) = wiring.sent.display.try_recv()
    else {
        panic!("expected the popup to be shown");
    };
    assert_eq!((buffer.width(), buffer.height()), (100, 40));
    assert_eq!(logical_size, Size::new(50, 20));
    assert_eq!(anchor_rect, Some(anchor));
}
