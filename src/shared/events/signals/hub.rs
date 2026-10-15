use std::collections::HashMap;

use chrono::{DateTime, Local};
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

pub type ModuleSizesMap = HashMap<MonitorId, ChildSizesMap>;
pub type MonitorScalesMap = HashMap<MonitorId, Scale>;
/// Terminal lazy-spawn failures, keyed by embedding site — module-not-found,
/// `init()` failure, or a cycle.
///
/// Broadcast the same way `module_sizes` is: every module actor reads the
/// whole map and filters to its own children, so a fix (hot reload clearing
/// an entry) wakes the render loop exactly like a size change does.
pub type ModuleErrorsMap = HashMap<ModuleSite, String>;

pub struct SignalHub {
    config: (watch::Sender<Config>, watch::Receiver<Config>),
    hyprland: (watch::Sender<HyprlandState>, watch::Receiver<HyprlandState>),
    time: (
        watch::Sender<DateTime<Local>>,
        watch::Receiver<DateTime<Local>>,
    ),
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
    #[must_use]
    pub fn new(initial_config: Config) -> Self {
        let config = watch::channel(initial_config);
        let hyprland = watch::channel(HyprlandState::new(
            std::collections::BTreeMap::new(),
            std::collections::BTreeMap::new(),
            None,
        ));
        let time = watch::channel(Local::now());
        let dbus = watch::channel(DBusState::default());
        let systray = watch::channel(SystrayState::default());
        let metrics = watch::channel(MetricsState::default());
        let mpris = watch::channel(MprisState::default());
        let pointer = tokio::sync::broadcast::channel(32);
        let module_sizes = watch::channel(HashMap::new());
        let monitor_scales = watch::channel(HashMap::new());
        let module_errors = watch::channel(HashMap::new());

        Self {
            config,
            hyprland,
            time,
            dbus,
            systray,
            metrics,
            pointer,
            mpris,
            module_sizes,
            monitor_scales,
            module_errors,
        }
    }

    #[must_use]
    pub fn module_errors_tx(&self) -> watch::Sender<ModuleErrorsMap> {
        self.module_errors.0.clone()
    }

    #[must_use]
    pub fn module_errors_rx(&self) -> watch::Receiver<ModuleErrorsMap> {
        self.module_errors.1.clone()
    }

    #[must_use]
    pub fn monitor_scales_tx(&self) -> watch::Sender<MonitorScalesMap> {
        self.monitor_scales.0.clone()
    }

    #[must_use]
    pub fn monitor_scales_rx(&self) -> watch::Receiver<MonitorScalesMap> {
        self.monitor_scales.1.clone()
    }

    #[must_use]
    pub fn module_sizes_tx(&self) -> watch::Sender<ModuleSizesMap> {
        self.module_sizes.0.clone()
    }

    #[must_use]
    pub fn module_sizes_rx(&self) -> watch::Receiver<ModuleSizesMap> {
        self.module_sizes.1.clone()
    }

    #[must_use]
    pub fn mpris_tx(&self) -> watch::Sender<MprisState> {
        self.mpris.0.clone()
    }

    #[must_use]
    pub fn mpris_rx(&self) -> watch::Receiver<MprisState> {
        self.mpris.1.clone()
    }

    #[must_use]
    pub fn config_tx(&self) -> watch::Sender<Config> {
        self.config.0.clone()
    }

    #[must_use]
    pub fn config_rx(&self) -> watch::Receiver<Config> {
        self.config.1.clone()
    }

    #[must_use]
    pub fn hyprland_tx(&self) -> watch::Sender<HyprlandState> {
        self.hyprland.0.clone()
    }

    #[must_use]
    pub fn hyprland_rx(&self) -> watch::Receiver<HyprlandState> {
        self.hyprland.1.clone()
    }

    #[must_use]
    pub fn time_tx(&self) -> watch::Sender<DateTime<Local>> {
        self.time.0.clone()
    }

    #[must_use]
    pub fn time_rx(&self) -> watch::Receiver<DateTime<Local>> {
        self.time.1.clone()
    }

    #[must_use]
    pub fn dbus_tx(&self) -> watch::Sender<DBusState> {
        self.dbus.0.clone()
    }

    #[must_use]
    pub fn dbus_rx(&self) -> watch::Receiver<DBusState> {
        self.dbus.1.clone()
    }

    #[must_use]
    pub fn systray_tx(&self) -> watch::Sender<SystrayState> {
        self.systray.0.clone()
    }

    #[must_use]
    pub fn systray_rx(&self) -> watch::Receiver<SystrayState> {
        self.systray.1.clone()
    }

    #[must_use]
    pub fn metrics_tx(&self) -> watch::Sender<MetricsState> {
        self.metrics.0.clone()
    }

    #[must_use]
    pub fn metrics_rx(&self) -> watch::Receiver<MetricsState> {
        self.metrics.1.clone()
    }

    #[must_use]
    pub const fn pointer_tx(&self) -> &PointerSender {
        &self.pointer.0
    }

    #[must_use]
    pub fn pointer_rx(&self) -> PointerReceiver {
        self.pointer.0.subscribe()
    }
}
