//! What dispatching one render outcome sends to the embedder and the display: layout events,
//! and floating content at the scale it was rendered at.
use super::lifecycle::make_test_rect_node;
use super::outcome::event_loop;
use crate::features::layout_engine::domain::{DisplayCommand, ShowFloating};
use crate::features::module_runtime::application::ModuleContext;
use crate::features::module_runtime::domain::render_pipeline::{EmbedReport, PipelineLayout};
use crate::features::module_runtime::domain::{
    FloatingLayouts, PopupRenderLayout, RenderOutcome, SizeChange,
};
use crate::features::module_runtime::ports::LayoutEvent;
use crate::features::module_runtime::test_support::{
    ChannelDisplaySender, ChannelLayoutSender, MockDisplaySender, MockLayoutSender, MockUiSender,
    RecordingSurfaceManager,
};
use crate::shared::config::domain::Config;
use crate::shared::events::signals::SignalHub;
use crate::shared::primitives::geometry::{Position, Rect, Scale, Size};
use crate::shared::primitives::{
    ChildBounds, ChildModuleLayout, LayoutSurface, ModuleId, ModuleKey, ModuleName, MonitorId,
    SizeConstraint,
};
use std::collections::HashMap;
use std::sync::Arc;

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

/// Child layouts go out while there are any, once more empty when the last one goes, then not
/// at all; a size change goes out with the settled width.
#[test]
fn layout_events_carry_the_module_monitor_children_and_settled_size() {
    let monitor = MonitorId::new("HDMI-A-1");
    let (layout_tx, layout_rx) = std::sync::mpsc::channel();
    let (_tx, rx) = tokio::sync::watch::channel(HashMap::new());
    let ctx = ModuleContext::new(
        ModuleId::new(42),
        Arc::new(SignalHub::new(Config::default())),
        Arc::new(RecordingSurfaceManager::default()),
        Arc::new(ChannelLayoutSender::new(layout_tx)),
        Arc::new(MockDisplaySender),
        Arc::new(MockUiSender),
        rx,
    );
    let mut event_loop = event_loop(ctx);
    let grown = SizeChange::new(Size::new(0, 0), Size::new(64, 22)).with_settled_width(Some(280));

    let with_child = RenderOutcome::new(
        PipelineLayout::new(
            make_test_rect_node(),
            FloatingLayouts::default(),
            EmbedReport::new(Some(grown), vec![clock_layout()]),
        ),
        None,
        Scale::new(1.0),
    );
    event_loop.dispatch_render_outcome(&monitor, &with_child);
    let empty = RenderOutcome::new(
        PipelineLayout::new(
            make_test_rect_node(),
            FloatingLayouts::default(),
            EmbedReport::new(None, vec![]),
        ),
        None,
        Scale::new(1.0),
    );
    event_loop.dispatch_render_outcome(&monitor, &empty);
    event_loop.dispatch_render_outcome(&monitor, &empty);
    event_loop.dispatch_render_outcome(&MonitorId::new("DP-1"), &empty);

    assert_eq!(
        layout_rx.try_iter().collect::<Vec<_>>(),
        vec![
            LayoutEvent::ContainerLayoutsCalculated {
                parent_id: ModuleId::new(42),
                monitor_id: monitor.clone(),
                layouts: vec![clock_layout()],
            },
            LayoutEvent::ModuleSizeChanged {
                monitor_id: monitor.clone(),
                module_id: ModuleId::new(42),
                size: Size::new(64, 22),
                settled_width: Some(280),
            },
            LayoutEvent::ContainerLayoutsCalculated {
                parent_id: ModuleId::new(42),
                monitor_id: monitor,
                layouts: vec![],
            },
        ]
    );
}

/// Floating content is painted at the scale the outcome was dispatched with.
#[test]
fn floating_content_is_painted_at_the_render_scale() {
    let (display_tx, display_rx) = std::sync::mpsc::channel();
    let (_tx, rx) = tokio::sync::watch::channel(HashMap::new());
    let ctx = ModuleContext::new(
        ModuleId::new(42),
        Arc::new(SignalHub::new(Config::default())),
        Arc::new(RecordingSurfaceManager::default()),
        Arc::new(MockLayoutSender),
        Arc::new(ChannelDisplaySender::new(display_tx)),
        Arc::new(MockUiSender),
        rx,
    );
    let anchor = Rect::new(Position::new(8, 0), Size::new(16, 20));
    let popup = PopupRenderLayout::new(anchor, make_test_rect_node(), None);

    event_loop(ctx).dispatch_render_outcome(
        &MonitorId::new("DP-1"),
        &RenderOutcome::new(
            PipelineLayout::new(
                make_test_rect_node(),
                FloatingLayouts::new(Some(popup), None, None),
                EmbedReport::new(None, vec![]),
            ),
            None,
            Scale::new(2.0),
        ),
    );

    let Ok(DisplayCommand::ShowFloatingSurface(ShowFloating {
        buffer,
        logical_size,
        anchor_rect,
        ..
    })) = display_rx.try_recv()
    else {
        panic!("expected the popup to be shown");
    };
    assert_eq!((buffer.width(), buffer.height()), (100, 40));
    assert_eq!(logical_size, Size::new(50, 20));
    assert_eq!(anchor_rect, Some(anchor));
}
