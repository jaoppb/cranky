//! The app, channels and layout values the `CrankyApp` tests are built from.

use crate::app::commands::SystemCommand;
use crate::app::registry::WatchLayoutSender;
use crate::app::state::{
    AppError, BusAdapters, CrankyApp, ModuleOnMonitor, ModuleReceivers, StateChannels,
};
use crate::app::test_fakes::FakeRegistry;
use crate::features::layout_engine::domain::{DisplayCommand, FloatingKind, ShowFloating};
use crate::features::module_runtime::ports::{
    LayoutEvent, ModuleHandles, ModuleRuntimeDependencies, ModuleSenders,
};
use crate::features::module_runtime::test_support::MockSurfaceManager;
use crate::features::vdom::domain::UiCommand;
use crate::shared::config::domain::Config;
use crate::shared::events::signals::SignalHub;
use crate::shared::primitives::geometry::{Position, Rect, Size};
use crate::shared::primitives::{
    ChildBounds, ChildModuleLayout, LayoutSurface, ModuleId, ModuleKey, ModuleName, MonitorId,
    SizeConstraint,
};
use crate::shared::rendering::adapters::tiny_skia::TinySkiaCanvasFactory;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, watch};

/// The app the tests drive, over the fake registry.
pub(super) type TestApp = CrankyApp<
    FakeRegistry,
    TinySkiaCanvasFactory,
    mpsc::Sender<LayoutEvent>,
    mpsc::Sender<DisplayCommand>,
    mpsc::Sender<UiCommand>,
    WatchLayoutSender,
>;

/// The sending ends of the four channels the app drains.
pub(super) struct Queues {
    pub(super) display: mpsc::Sender<DisplayCommand>,
    pub(super) layout: mpsc::Sender<LayoutEvent>,
    pub(super) interface: mpsc::Sender<UiCommand>,
    pub(super) system: mpsc::Sender<SystemCommand>,
}

/// An app, the hub it reads, and the sending ends of the four channels it drains.
pub(super) struct Harness {
    pub(super) app: TestApp,
    pub(super) hub: Arc<SignalHub>,
    pub(super) queues: Queues,
}

/// An app over `registry`, its hub started from `config`, on fresh channels whose sending ends
/// the test keeps.
pub(super) fn harness(config: Config, registry: FakeRegistry) -> Harness {
    try_harness(config, registry).unwrap()
}

/// What `CrankyApp::new` makes of `registry` and `config`: a harness, or the error it fails with.
pub(super) fn try_harness(config: Config, registry: FakeRegistry) -> Result<Harness, AppError> {
    let hub = Arc::new(SignalHub::new(config));
    let (queues, channels) = fresh_channels();
    let senders = ModuleSenders::new(
        Arc::new(queues.layout.clone()),
        Arc::new(queues.display.clone()),
        Arc::new(queues.interface.clone()),
    );
    let deps = ModuleRuntimeDependencies::new(
        ModuleHandles::new(hub.clone(), Arc::new(MockSurfaceManager), senders),
        TinySkiaCanvasFactory::new(),
    );
    let app = CrankyApp::new(deps, channels, Box::new(registry))?;
    Ok(Harness { app, hub, queues })
}

/// Four fresh channels: their sending ends, and their receiving ends as the app takes them.
fn fresh_channels() -> (Queues, StateChannels) {
    let (display_tx, display_rx) = mpsc::channel(32);
    let (layout_tx, layout_rx) = mpsc::channel(32);
    let (ui_tx, ui_rx) = mpsc::channel(32);
    let (system_tx, system_rx) = mpsc::channel(32);
    let queues = Queues {
        display: display_tx,
        layout: layout_tx,
        interface: ui_tx,
        system: system_tx,
    };
    let receivers = ModuleReceivers::new(layout_rx, display_rx, ui_rx);
    (queues, StateChannels::new(receivers, system_rx))
}

/// An app over `registry`, its hub started from `config`, on fresh channels.
pub(super) fn app_over(config: Config, registry: FakeRegistry) -> (TestApp, Arc<SignalHub>) {
    let Harness { app, hub, .. } = harness(config, registry);
    (app, hub)
}

/// A configuration whose `[metrics]` table samples every 2.5 s instead of every second.
pub(super) fn config_sampling_every_2500_ms() -> Config {
    let metrics: crate::features::metrics::domain::MetricsConfig =
        toml::from_str("update_interval_ms = 2500").unwrap();
    Config::default().with_metrics(metrics)
}

/// The session-bus adapters `run` takes: subscriptions on a bus that is never connected, and
/// `systray`.
pub(super) fn bus_with<S>(hub: &Arc<SignalHub>, systray: S) -> BusAdapters<S> {
    let subscriptions = crate::shared::dbus::subscription_manager::DbusSubscriptionManager::new(
        Arc::new(crate::shared::dbus::ports::DisconnectedBus),
        hub,
    );
    BusAdapters::new(subscriptions, systray)
}

/// The key of the module called `name`, with no instance id.
pub(super) fn key(name: &str) -> ModuleKey {
    ModuleKey::from_name(ModuleName::new(name))
}

/// `module` on `monitor`.
pub(super) const fn module_on(module: ModuleId, monitor: &MonitorId) -> ModuleOnMonitor<'_> {
    ModuleOnMonitor { module, monitor }
}

/// A child laid out at `key`, 50 by 20 at (`x`, 0), on `surface`.
pub(super) fn laid_out(key: ModuleKey, x: i32, surface: LayoutSurface) -> ChildModuleLayout {
    ChildModuleLayout::new(
        key,
        ChildBounds::new(
            Rect::new(Position::new(x, 0), Size::new(50, 20)),
            SizeConstraint::none(),
        ),
        surface,
    )
}

/// A request to show the tooltip, one pixel big, anchored nowhere.
pub(super) fn show_tooltip() -> DisplayCommand {
    DisplayCommand::ShowFloatingSurface(ShowFloating {
        kind: FloatingKind::Tooltip,
        monitor_id: None,
        anchor_rect: None,
        buffer: crate::shared::primitives::render::RenderBuffer::new(vec![0u8; 4], Size::new(1, 1)),
        logical_size: Size::new(1, 1),
        offset: None,
        parent: None,
    })
}

/// A request to hide the tooltip.
pub(super) const fn hide_tooltip() -> DisplayCommand {
    DisplayCommand::HideFloatingSurface {
        kind: FloatingKind::Tooltip,
    }
}

/// A request to trigger `action` on the systray item `id`, with no pointer position.
pub(super) fn systray_action(id: &str, action: &str) -> UiCommand {
    UiCommand::SystrayAction {
        id: id.into(),
        action: action.into(),
        pos: None,
    }
}

/// What `calls` holds now, taken out of it.
pub(super) fn drain(calls: &Arc<std::sync::Mutex<Vec<String>>>) -> Vec<String> {
    std::mem::take(&mut *calls.lock().unwrap())
}

/// Sends every item of `items` on `tx`, in order.
pub(super) async fn send_all<T>(tx: &mpsc::Sender<T>, items: impl IntoIterator<Item = T>) {
    for item in items {
        tx.send(item).await.unwrap();
    }
}

/// A registry holding a root `bar` (id 0) that embeds one `clock` (id 1).
pub(super) fn bar_embedding_clock() -> FakeRegistry {
    let mut registry = FakeRegistry::default();
    let (bar, clock) = (ModuleName::new("bar"), ModuleName::new("clock"));
    registry.root_module = Some(ModuleId::new(0));
    registry.module_ids = vec![ModuleId::new(0), ModuleId::new(1)];
    registry.module_names = HashMap::from([
        (ModuleId::new(0), bar.clone()),
        (ModuleId::new(1), clock.clone()),
    ]);
    registry.name_to_ids = HashMap::from([
        (bar, vec![ModuleId::new(0)]),
        (clock, vec![ModuleId::new(1)]),
    ]);
    registry.resolve_site = Box::new(|parent, key| {
        (parent == Some(ModuleId::new(0)) && key.name().as_str() == "clock")
            .then_some(ModuleId::new(1))
    });
    registry
}

/// The bar reporting where it laid its clock out on `monitor`.
pub(super) fn clock_laid_out_on(monitor: &str, x: i32) -> LayoutEvent {
    LayoutEvent::ContainerLayoutsCalculated {
        parent_id: ModuleId::new(0),
        monitor_id: MonitorId::new(monitor),
        layouts: vec![ChildModuleLayout::new(
            key("clock"),
            ChildBounds::new(
                Rect::new(Position::new(x, 0), Size::new(80, 24)),
                SizeConstraint::none(),
            ),
            LayoutSurface::Bar,
        )],
    }
}

/// The bounds a module is sent, one entry per monitor it is shown on.
pub(super) type BoundsByMonitor = HashMap<MonitorId, ChildBounds>;

/// A layout sender a test hands the app for a module, and the end the test reads it from.
pub(super) fn layout_channel() -> (WatchLayoutSender, watch::Receiver<BoundsByMonitor>) {
    let (layout_tx, layout_rx) = watch::channel(HashMap::new());
    (WatchLayoutSender::new(layout_tx), layout_rx)
}

/// The lines of `logs` logged at the error level.
pub(super) fn errors_in(logs: &str) -> Vec<&str> {
    logs.lines()
        .filter(|line| line.starts_with("ERROR"))
        .collect()
}
