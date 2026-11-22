//! The `SignalHub` and the map types it broadcasts.

use std::collections::HashMap;

use cranky_time::{Clock, LocalTime};
use tokio::sync::watch;

use crate::features::metrics::domain::MetricsState;
use crate::features::mpris::domain::MprisState;
use crate::features::systray::domain::SystrayState;
use crate::shared::config::domain::Config;
use crate::shared::dbus::domain::DBusState;
use crate::shared::events::core::{PointerReceiver, PointerSender};
use crate::shared::primitives::geometry::Scale;
use crate::shared::primitives::{ChildSizesMap, ModuleSite, MonitorId};

use super::hyprland::HyprlandState;

pub(crate) type ModuleSizesMap = HashMap<MonitorId, ChildSizesMap>;
pub(crate) type MonitorScalesMap = HashMap<MonitorId, Scale>;
/// Terminal lazy-spawn failures, keyed by embedding site — module-not-found,
/// `init()` failure, or a cycle.
///
/// Broadcast the same way `module_sizes` is: every module actor reads the
/// whole map and filters to its own children, so a fix (hot reload clearing
/// an entry) wakes the render loop exactly like a size change does.
pub(crate) type ModuleErrorsMap = HashMap<ModuleSite, String>;

/// Holds one `watch` channel per state source plus the pointer broadcast; adapters write to the senders, and module actors wake on the receivers of the signals they subscribe to.
#[derive(Debug)]
pub struct SignalHub {
    config: (watch::Sender<Config>, watch::Receiver<Config>),
    hyprland: (watch::Sender<HyprlandState>, watch::Receiver<HyprlandState>),
    time: (watch::Sender<LocalTime>, watch::Receiver<LocalTime>),
    dbus: (watch::Sender<DBusState>, watch::Receiver<DBusState>),
    systray: (watch::Sender<SystrayState>, watch::Receiver<SystrayState>),
    metrics: (watch::Sender<MetricsState>, watch::Receiver<MetricsState>),
    pointer: (PointerSender, PointerReceiver),
    mpris: (watch::Sender<MprisState>, watch::Receiver<MprisState>),
    module_sizes: (
        watch::Sender<ModuleSizesMap>,
        watch::Receiver<ModuleSizesMap>,
    ),
    monitor_scales: (
        watch::Sender<MonitorScalesMap>,
        watch::Receiver<MonitorScalesMap>,
    ),
    module_errors: (
        watch::Sender<ModuleErrorsMap>,
        watch::Receiver<ModuleErrorsMap>,
    ),
}

impl SignalHub {
    /// Builds the hub with `initial_config`, the current local time and empty state for every other source.
    #[must_use]
    pub fn new(initial_config: Config) -> Self {
        Self {
            // The state sources adapters publish and module scripts subscribe to.
            config: watch::channel(initial_config),
            hyprland: watch::channel(HyprlandState::new(
                std::collections::BTreeMap::new(),
                std::collections::BTreeMap::new(),
                None,
            )),
            time: watch::channel(Clock::new().now()),
            dbus: watch::channel(DBusState::default()),
            systray: watch::channel(SystrayState::default()),
            metrics: watch::channel(MetricsState::default()),
            mpris: watch::channel(MprisState::default()),
            // Pointer interactions, broadcast to the module actors.
            pointer: tokio::sync::broadcast::channel(32),
            // What module actors and the Wayland side report to each other, per monitor and site.
            module_sizes: watch::channel(HashMap::new()),
            monitor_scales: watch::channel(HashMap::new()),
            module_errors: watch::channel(HashMap::new()),
        }
    }

    #[must_use]
    pub(crate) fn module_errors_tx(&self) -> watch::Sender<ModuleErrorsMap> {
        self.module_errors.0.clone()
    }

    #[must_use]
    pub(crate) fn module_errors_rx(&self) -> watch::Receiver<ModuleErrorsMap> {
        self.module_errors.1.clone()
    }

    #[must_use]
    pub(crate) fn monitor_scales_tx(&self) -> watch::Sender<MonitorScalesMap> {
        self.monitor_scales.0.clone()
    }

    /// A receiver for the scale of every monitor Wayland has announced; its keys are the monitors surfaces can be created on.
    #[must_use]
    pub fn monitor_scales_rx(&self) -> watch::Receiver<MonitorScalesMap> {
        self.monitor_scales.1.clone()
    }

    #[must_use]
    pub(crate) fn module_sizes_tx(&self) -> watch::Sender<ModuleSizesMap> {
        self.module_sizes.0.clone()
    }

    #[must_use]
    pub(crate) fn module_sizes_rx(&self) -> watch::Receiver<ModuleSizesMap> {
        self.module_sizes.1.clone()
    }

    #[must_use]
    pub(crate) fn mpris_tx(&self) -> watch::Sender<MprisState> {
        self.mpris.0.clone()
    }

    #[must_use]
    pub(crate) fn mpris_rx(&self) -> watch::Receiver<MprisState> {
        self.mpris.1.clone()
    }

    #[must_use]
    pub(crate) fn config_tx(&self) -> watch::Sender<Config> {
        self.config.0.clone()
    }

    #[must_use]
    pub(crate) fn config_rx(&self) -> watch::Receiver<Config> {
        self.config.1.clone()
    }

    #[must_use]
    pub(crate) fn hyprland_tx(&self) -> watch::Sender<HyprlandState> {
        self.hyprland.0.clone()
    }

    #[must_use]
    pub(crate) fn hyprland_rx(&self) -> watch::Receiver<HyprlandState> {
        self.hyprland.1.clone()
    }

    #[must_use]
    pub(crate) fn time_tx(&self) -> watch::Sender<LocalTime> {
        self.time.0.clone()
    }

    #[must_use]
    pub(crate) fn time_rx(&self) -> watch::Receiver<LocalTime> {
        self.time.1.clone()
    }

    #[must_use]
    pub(crate) fn dbus_tx(&self) -> watch::Sender<DBusState> {
        self.dbus.0.clone()
    }

    #[must_use]
    pub(crate) fn dbus_rx(&self) -> watch::Receiver<DBusState> {
        self.dbus.1.clone()
    }

    #[must_use]
    pub(crate) fn systray_tx(&self) -> watch::Sender<SystrayState> {
        self.systray.0.clone()
    }

    /// A receiver for the tray items, which the scripting engines read into `signals.systray`.
    #[must_use]
    pub fn systray_rx(&self) -> watch::Receiver<SystrayState> {
        self.systray.1.clone()
    }

    #[must_use]
    pub(crate) fn metrics_tx(&self) -> watch::Sender<MetricsState> {
        self.metrics.0.clone()
    }

    #[must_use]
    pub(crate) fn metrics_rx(&self) -> watch::Receiver<MetricsState> {
        self.metrics.1.clone()
    }

    #[must_use]
    pub(crate) const fn pointer_tx(&self) -> &PointerSender {
        &self.pointer.0
    }

    #[must_use]
    pub(crate) fn pointer_rx(&self) -> PointerReceiver {
        self.pointer.0.subscribe()
    }
}
