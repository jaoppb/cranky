//! An event loop for module 42 wired to channels and recorders, for the tests that drive it and
//! read everything it sends.
use crate::features::layout_engine::domain::DisplayCommand;
use crate::features::module_runtime::adapters::event_loop::runner::EventLoop;
use crate::features::module_runtime::application::ModuleContext;
use crate::features::module_runtime::domain::RenderServices;
use crate::features::module_runtime::ports::{
    AnyModulePort, LayoutEvent, ModuleHandles, ModuleSenders,
};
use crate::features::module_runtime::test_support::{
    ChannelDisplaySender, ChannelLayoutSender, ChannelUiSender, MockCanvasFactory,
    RecordingSurfaceManager,
};
use crate::features::styling::adapters::fs_loader::CompositeStyleResolver;
use crate::features::vdom::adapters::DefaultVdomDiffAdapter;
use crate::features::vdom::domain::UiCommand;
use crate::shared::config::domain::Config;
use crate::shared::events::signals::SignalHub;
use crate::shared::primitives::geometry::Scale;
use crate::shared::primitives::{ChildBounds, ModuleId, MonitorId};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, channel};
use tokio::sync::watch;

/// The module every fixture event loop runs.
pub(in super::super) const MODULE: ModuleId = ModuleId::new(42);

/// An event loop whose senders all write to channels a test reads.
pub(in super::super) type ChannelEventLoop =
    EventLoop<MockCanvasFactory, ChannelLayoutSender, ChannelDisplaySender, ChannelUiSender>;

/// The receiving ends of everything a fixture event loop sends.
pub(in super::super) struct Sent {
    pub(in super::super) layout_events: Receiver<LayoutEvent>,
    pub(in super::super) ui_commands: Receiver<UiCommand>,
}

/// The ends of a fixture event loop a test feeds and reads.
pub(in super::super) struct Wiring {
    pub(in super::super) hub: Arc<SignalHub>,
    /// The bounds the parent gives the module, per monitor.
    pub(in super::super) layouts: watch::Sender<HashMap<MonitorId, ChildBounds>>,
    pub(in super::super) sent: Sent,
    pub(in super::super) surfaces: Arc<RecordingSurfaceManager>,
}

/// A hub on which Wayland reports DP-1 at `scale`.
pub(in super::super) fn hub_with_dp1(scale: f32) -> Arc<SignalHub> {
    let hub = Arc::new(SignalHub::new(Config::default()));
    hub.monitor_scales_tx()
        .send_replace(HashMap::from([(MonitorId::new("DP-1"), Scale::new(scale))]));
    hub
}

/// Senders that write to channels, with those channels' receiving ends.
fn channel_senders() -> (
    ModuleSenders<ChannelLayoutSender, ChannelDisplaySender, ChannelUiSender>,
    Sent,
) {
    let (layout_tx, layout_events) = channel();
    let (display_tx, _) = channel::<DisplayCommand>();
    let (ui_tx, ui_commands) = channel();
    let senders = ModuleSenders::new(
        Arc::new(ChannelLayoutSender::new(layout_tx)),
        Arc::new(ChannelDisplaySender::new(display_tx)),
        Arc::new(ChannelUiSender { tx: ui_tx }),
    );
    let sent = Sent {
        layout_events,
        ui_commands,
    };
    (senders, sent)
}

/// The services of a module with no stylesheet, diffed by the default differ.
pub(in super::super) fn plain_services() -> RenderServices<MockCanvasFactory> {
    RenderServices::new(
        MockCanvasFactory,
        Arc::new(CompositeStyleResolver::new(vec![])),
        Arc::new(DefaultVdomDiffAdapter::new()),
    )
}

/// A top-level event loop for `MODULE` running `port` on `hub`, with every sender wired to a
/// channel and the surface manager recording each frame.
pub(in super::super) fn wired_event_loop(
    port: impl AnyModulePort + 'static,
    hub: Arc<SignalHub>,
) -> (ChannelEventLoop, Wiring) {
    let (senders, sent) = channel_senders();
    let (layouts, layout_rx) = watch::channel(HashMap::new());
    let surfaces = Arc::new(RecordingSurfaceManager::default());
    let handles = ModuleHandles::new(hub.clone(), surfaces.clone(), senders);
    let ctx = ModuleContext::new(MODULE, handles, layout_rx);
    let wiring = Wiring {
        hub,
        layouts,
        sent,
        surfaces,
    };
    (
        EventLoop::new(Box::new(port), ctx, plain_services()),
        wiring,
    )
}
