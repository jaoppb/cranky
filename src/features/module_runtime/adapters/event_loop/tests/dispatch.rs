use crate::features::module_runtime::adapters::event_loop::runner::EventLoop;
use crate::features::module_runtime::application::ModuleContext;
use crate::features::module_runtime::domain::{PointerHandler, RenderPipeline};
use crate::features::module_runtime::ports::LayoutEvent;
use crate::features::module_runtime::test_support::{
    ChannelLayoutSender, MockCanvasFactory, MockDisplaySender, MockLayoutSender,
    MockSurfaceManager, MockUiSender, TestModulePort,
};
use crate::features::styling::adapters::fs_loader::CompositeStyleResolver;
use crate::features::vdom::adapters::DefaultVdomDiffAdapter;
use crate::features::vdom::domain::VNode;
use crate::shared::config::domain::Config;
use crate::shared::events::signals::SignalHub;
use crate::shared::primitives::geometry::{Position, Rect, Size};
use crate::shared::primitives::ModuleId;
use crate::shared::primitives::MonitorId;
use std::collections::HashMap;
use std::sync::Arc;

#[test]
fn test_discover_monitors_aggregates_sources() {
    let id = ModuleId::new(1);
    let hub = Arc::new(SignalHub::new(Config::default()));
    let sm = Arc::new(MockSurfaceManager);
    let layout_sender = Arc::new(MockLayoutSender);
    let display_sender = Arc::new(MockDisplaySender);
    let ui_sender = Arc::new(MockUiSender);
    let (_tx, rx) = tokio::sync::watch::channel(HashMap::new());
    let ctx = ModuleContext::new(id, hub, sm, layout_sender, display_sender, ui_sender, rx);

    let event_loop = EventLoop::new(
        Box::new(TestModulePort::new(VNode::new_rect(
            None, None, None, None, None,
        ))),
        ctx,
        PointerHandler::new(),
        RenderPipeline::new(),
        MockCanvasFactory,
        Arc::new(CompositeStyleResolver::new(vec![])),
        Arc::new(DefaultVdomDiffAdapter::new()),
    );

    let mut layouts = HashMap::new();
    layouts.insert(
        MonitorId::new("HDMI-A-1"),
        Rect::new(Position::new(0, 0), Size::new(1920, 30)),
    );
    let discovered = event_loop.discover_monitors(&layouts);
    assert!(discovered.contains(&MonitorId::new("HDMI-A-1")));
}

#[tokio::test]
async fn test_dispatch_render_outcome_emits_layout_events() {
    use crate::features::module_runtime::domain::SizeChange;
    use crate::features::styling::domain::ComputedStyle;

    let id = ModuleId::new(1);
    let hub = Arc::new(SignalHub::new(Config::default()));
    let sm = Arc::new(MockSurfaceManager);
    let (layout_tx, layout_rx) = std::sync::mpsc::channel();
    let layout_sender = Arc::new(ChannelLayoutSender::new(layout_tx));
    let display_sender = Arc::new(MockDisplaySender);
    let ui_sender = Arc::new(MockUiSender);
    let (_tx, rx) = tokio::sync::watch::channel(HashMap::new());
    let ctx = ModuleContext::new(id, hub, sm, layout_sender, display_sender, ui_sender, rx);

    let mut event_loop = EventLoop::new(
        Box::new(TestModulePort::new(VNode::new_rect(
            None, None, None, None, None,
        ))),
        ctx,
        PointerHandler::new(),
        RenderPipeline::new(),
        MockCanvasFactory,
        Arc::new(CompositeStyleResolver::new(vec![])),
        Arc::new(DefaultVdomDiffAdapter::new()),
    );

    let outcome = crate::features::module_runtime::domain::RenderOutcome::new(
        Some(SizeChange::new(Size::new(0, 0), Size::new(50, 20))),
        vec![],
        crate::features::layout_engine::domain::RenderNode::Rect {
            path: crate::features::layout_engine::domain::NodePath::root(),
            rect: Rect::new(Position::new(0, 0), Size::new(50, 20)),
            style: ComputedStyle::default(),
            on_click: None,
            on_hover: None,
            tooltip: None,
            popup: None,
        },
        None,
    );

    event_loop.dispatch_render_outcome(&MonitorId::new("DP-1"), &outcome);

    let received = layout_rx
        .try_recv()
        .expect("Should have sent ModuleSizeChanged");
    match received {
        LayoutEvent::ModuleSizeChanged {
            monitor_id,
            module_id,
            size,
        } => {
            assert_eq!(monitor_id.as_str(), "DP-1");
            assert_eq!(module_id, id);
            assert_eq!(size, Size::new(50, 20));
        }
        _ => panic!("Unexpected event"),
    }
}

#[test]
fn test_handle_pointer_event_no_tree_returns_false() {
    let id = ModuleId::new(1);
    let hub = Arc::new(SignalHub::new(Config::default()));
    let sm = Arc::new(MockSurfaceManager);
    let layout_sender = Arc::new(MockLayoutSender);
    let display_sender = Arc::new(MockDisplaySender);
    let ui_sender = Arc::new(MockUiSender);
    let (_tx, rx) = tokio::sync::watch::channel(HashMap::new());
    let ctx = ModuleContext::new(id, hub, sm, layout_sender, display_sender, ui_sender, rx);

    let mut event_loop = EventLoop::new(
        Box::new(TestModulePort::new(VNode::new_rect(
            None, None, None, None, None,
        ))),
        ctx,
        PointerHandler::new(),
        RenderPipeline::new(),
        MockCanvasFactory,
        Arc::new(CompositeStyleResolver::new(vec![])),
        Arc::new(DefaultVdomDiffAdapter::new()),
    );

    let event = crate::shared::events::core::PointerEvent::Click {
        button: crate::shared::events::core::PointerButton::Left,
        pos: Position::new(10, 10),
    };
    let changed = event_loop.handle_pointer_event(&MonitorId::new("DP-1"), &event);
    assert!(!changed);
}
