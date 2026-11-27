//! The app, channels and layout values the `CrankyApp` tests are built from.

use crate::app::commands::SystemCommand;
use crate::app::registry::WatchLayoutSender;
use crate::app::state::{BusAdapters, CrankyApp, ModuleOnMonitor, ModuleReceivers, StateChannels};
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
use std::sync::Arc;
use tokio::sync::mpsc;

/// The app the tests drive, over the fake registry.
pub(super) type TestApp = CrankyApp<
    FakeRegistry,
    TinySkiaCanvasFactory,
    mpsc::Sender<LayoutEvent>,
    mpsc::Sender<DisplayCommand>,
    mpsc::Sender<UiCommand>,
    WatchLayoutSender,
>;

/// An app, the hub it reads, and the sending ends of the four channels it drains.
pub(super) struct Harness {
    pub(super) app: TestApp,
    pub(super) hub: Arc<SignalHub>,
    pub(super) display_tx: mpsc::Sender<DisplayCommand>,
    pub(super) layout_tx: mpsc::Sender<LayoutEvent>,
    pub(super) ui_tx: mpsc::Sender<UiCommand>,
    pub(super) system_tx: mpsc::Sender<SystemCommand>,
}

/// An app over `registry`, its hub started from `config`, on fresh channels whose sending ends
/// the test keeps.
pub(super) fn harness(config: Config, registry: FakeRegistry) -> Harness {
    let hub = Arc::new(SignalHub::new(config));
    let (display_tx, display_rx) = mpsc::channel(32);
    let (layout_tx, layout_rx) = mpsc::channel(32);
    let (ui_tx, ui_rx) = mpsc::channel(32);
    let (system_tx, system_rx) = mpsc::channel(32);
    let senders = ModuleSenders::new(
        Arc::new(layout_tx.clone()),
        Arc::new(display_tx.clone()),
        Arc::new(ui_tx.clone()),
    );
    let deps = ModuleRuntimeDependencies::new(
        ModuleHandles::new(hub.clone(), Arc::new(MockSurfaceManager), senders),
        TinySkiaCanvasFactory::new(),
    );
    let channels = StateChannels::new(
        ModuleReceivers::new(layout_rx, display_rx, ui_rx),
        system_rx,
    );
    let app = CrankyApp::new(deps, channels, Box::new(registry)).unwrap();
    Harness {
        app,
        hub,
        display_tx,
        layout_tx,
        ui_tx,
        system_tx,
    }
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
