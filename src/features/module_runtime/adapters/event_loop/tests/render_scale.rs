//! A render at a monitor's scale: the module's own buffer and the floating content the event loop
//! paints for it.
use super::outcome::event_loop;
use crate::features::layout_engine::adapters::taffy::TaffyLayoutAdapter;
use crate::features::layout_engine::domain::{DisplayCommand, ShowFloating};
use crate::features::module_runtime::application::ModuleContext;
use crate::features::module_runtime::domain::RenderPipeline;
use crate::features::module_runtime::ports::{ModuleHandles, ModuleSenders};
use crate::features::module_runtime::test_support::{
    ChannelDisplaySender, MockLayoutSender, MockUiSender, PipelineFixture, RecordingSurfaceManager,
    TestModulePort, process_monitor,
};
use crate::features::vdom::domain::{PopupSpec, TextContent, VNode};
use crate::shared::config::domain::Config;
use crate::shared::events::signals::SignalHub;
use crate::shared::primitives::geometry::{Position, Rect, Scale, Size};
use crate::shared::primitives::{ModuleId, MonitorId};
use std::collections::HashMap;
use std::sync::Arc;

/// A render on a monitor at scale 2 paints the module's own buffer at that scale, and the event
/// loop paints the floating content it opened at the same scale.
#[test]
fn a_render_at_scale_two_paints_its_buffer_and_its_popup_at_that_scale() {
    let monitor = MonitorId::new("DP-1");
    let text = |label: &str| {
        VNode::new_text(
            TextContent::new(label.to_string()),
            None,
            None,
            None,
            None,
            None,
        )
    };
    let bar = text("bar").with_popup(PopupSpec::new(Box::new(text("calendar"))));
    let mut pipeline = RenderPipeline::new();
    let mut fixture = PipelineFixture::<TaffyLayoutAdapter>::default();
    let mut ctx = fixture.ctx(&monitor);
    ctx.scale = Scale::new(2.0);
    ctx.current_bounds = Some(Rect::new(Position::new(0, 0), Size::new(10, 10)));

    let outcome =
        process_monitor(&mut pipeline, &TestModulePort::new(bar), ctx).expect("render outcome");
    let (buffer, _) = outcome.buffer().expect("painted buffer");
    assert_eq!((buffer.width(), buffer.height()), (20, 20));

    let (display_tx, display_rx) = std::sync::mpsc::channel();
    let (_tx, rx) = tokio::sync::watch::channel(HashMap::new());
    let ctx = ModuleContext::new(
        ModuleId::new(42),
        ModuleHandles::new(
            Arc::new(SignalHub::new(Config::default())),
            Arc::new(RecordingSurfaceManager::default()),
            ModuleSenders::new(
                Arc::new(MockLayoutSender),
                Arc::new(ChannelDisplaySender::new(display_tx)),
                Arc::new(MockUiSender),
            ),
        ),
        rx,
    );
    event_loop(ctx).dispatch_render_outcome(&monitor, &outcome);

    let Ok(DisplayCommand::ShowFloatingSurface(ShowFloating {
        buffer,
        logical_size,
        ..
    })) = display_rx.try_recv()
    else {
        panic!("expected the popup to be shown");
    };
    assert_eq!(
        (buffer.width(), buffer.height()),
        (logical_size.width() * 2, logical_size.height() * 2)
    );
    assert!(logical_size.width() > 0);
}
