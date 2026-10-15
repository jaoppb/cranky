use crate::features::layout_engine::adapters::taffy::TaffyLayoutAdapter;
use crate::features::module_runtime::adapters::event_loop::runner::EventLoop;
use crate::features::module_runtime::application::ModuleContext;
use crate::features::module_runtime::domain::{PointerHandler, RenderPipeline};
use crate::features::module_runtime::test_support::{
    MockCanvasFactory, MockDisplaySender, MockLayoutSender, MockUiSender, TestModulePort,
};
use crate::features::styling::adapters::fs_loader::CompositeStyleResolver;
use crate::features::vdom::adapters::DefaultVdomDiffAdapter;
use crate::features::vdom::domain::VNode;
use crate::shared::config::domain::Config;
use crate::shared::events::signals::SignalHub;
use crate::shared::primitives::geometry::{Position, Rect, Scale, Size};
use crate::shared::primitives::render::RenderBuffer;
use crate::shared::primitives::{ChildBounds, LayoutSurface, ModuleId, MonitorId, SizeConstraint};
use crate::shared::wayland::ports::{SurfaceManagerPort, SurfaceParent};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Counts every `submit_child_buffer` — the only observable proof that an
/// embedded actor actually re-painted and handed its pixels to Wayland.
#[derive(Default)]
struct CountingSurfaceManager {
    child_submits: AtomicUsize,
}

impl SurfaceManagerPort for CountingSurfaceManager {
    fn submit_buffer(&self, _: ModuleId, _: MonitorId, _: Position, _: RenderBuffer) {}

    fn submit_child_buffer(
        &self,
        _: ModuleId,
        _: Option<SurfaceParent>,
        _: MonitorId,
        _: Position,
        _: RenderBuffer,
    ) {
        self.child_submits.fetch_add(1, Ordering::SeqCst);
    }
}

type Layouts = HashMap<MonitorId, ChildBounds>;
type TestEventLoop =
    EventLoop<MockCanvasFactory, MockLayoutSender, MockDisplaySender, MockUiSender>;

fn embedded_event_loop(
    mon: &MonitorId,
) -> (
    TestEventLoop,
    tokio::sync::watch::Sender<Layouts>,
    Arc<CountingSurfaceManager>,
) {
    let hub = Arc::new(SignalHub::new(Config::default()));
    let mut scales = hub.monitor_scales_rx().borrow().clone();
    scales.insert(mon.clone(), Scale::new(1.0));
    let _ = hub.monitor_scales_tx().send(scales);

    let sm = Arc::new(CountingSurfaceManager::default());
    let (layout_tx, layout_rx) = tokio::sync::watch::channel(Layouts::new());
    // Embedded inside module 7's popup — the calendar-in-clock-popup case.
    let ctx = ModuleContext::new(
        ModuleId::new(42),
        hub,
        sm.clone(),
        Arc::new(MockLayoutSender),
        Arc::new(MockDisplaySender),
        Arc::new(MockUiSender),
        layout_rx,
    )
    .with_parent(Some(ModuleId::new(7)))
    .with_surface(LayoutSurface::Popup);

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
    (event_loop, layout_tx, sm)
}

fn site(mon: &MonitorId) -> Layouts {
    let rect = Rect::new(Position::new(4, 6), Size::new(40, 30));
    let mut layouts = Layouts::new();
    layouts.insert(mon.clone(), ChildBounds::new(rect, SizeConstraint::none()));
    layouts
}

/// The popup embedding this actor closes (its rect is withdrawn) and then
/// reopens with the very same rect. The popup's surface — and the child's
/// pixels composited into it — were torn down on close, so the reopened
/// slot is a hole unless this actor re-submits its buffer, even though its
/// VDOM, rect, child sizes and interaction state are all unchanged.
#[test]
fn suspended_embedded_actor_rerenders_when_site_returns() {
    let mon = MonitorId::new("DP-1");
    let (mut event_loop, layout_tx, sm) = embedded_event_loop(&mon);
    let mut engines: HashMap<MonitorId, TaffyLayoutAdapter> = HashMap::new();

    let _ = layout_tx.send(site(&mon));
    event_loop.render_all_monitors(&mut engines);
    assert_eq!(sm.child_submits.load(Ordering::SeqCst), 1);

    // Popup closes: the parent withdraws the rect. Several passes happen
    // while suspended (any subscribed signal wakes the actor).
    let _ = layout_tx.send(Layouts::new());
    event_loop.render_all_monitors(&mut engines);
    event_loop.render_all_monitors(&mut engines);
    event_loop.render_all_monitors(&mut engines);
    assert_eq!(sm.child_submits.load(Ordering::SeqCst), 1);

    // Popup reopens at the same rect. Relies on `retain_monitors` having
    // cleared the suspended monitor's rendered bounds: a pruning change that
    // keeps them would make this an unchanged no-op and leave a permanent
    // hole in the composited popup.
    let _ = layout_tx.send(site(&mon));
    event_loop.render_all_monitors(&mut engines);
    assert_eq!(sm.child_submits.load(Ordering::SeqCst), 2);
}
