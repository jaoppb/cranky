use serde::{Serialize, Deserialize};
use crate::domain::config::Config;
use crate::domain::workspace::{Workspace, Monitor};
use crate::domain::ModuleId;
use tokio::sync::{watch, mpsc};

use crate::domain::dbus::{DBusState, DBusSubscription};
use crate::domain::applets::AppletsState;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SignalKind {
    Time,
    Hyprland,
    DBus(DBusSubscription),
    Applets,
    Metrics,
}


#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct HyprlandState {
    workspaces: Vec<Workspace>,
    monitors: Vec<Monitor>,
}

impl HyprlandState {
    pub fn new(workspaces: Vec<Workspace>, monitors: Vec<Monitor>) -> Self {
        Self { workspaces, monitors }
    }

    pub fn workspaces(&self) -> &[Workspace] {
        &self.workspaces
    }

    pub fn monitors(&self) -> &[Monitor] {
        &self.monitors
    }
}

pub struct SignalHub {
    config: (watch::Sender<Config>, watch::Receiver<Config>),
    hyprland: (watch::Sender<HyprlandState>, watch::Receiver<HyprlandState>),
    time: (watch::Sender<chrono::DateTime<chrono::Local>>, watch::Receiver<chrono::DateTime<chrono::Local>>),
    dbus: (watch::Sender<DBusState>, watch::Receiver<DBusState>),
    applets: (watch::Sender<AppletsState>, watch::Receiver<AppletsState>),
    metrics: (watch::Sender<crate::domain::metrics::MetricsState>, watch::Receiver<crate::domain::metrics::MetricsState>),
}

impl SignalHub {
    pub fn new(initial_config: Config) -> Self {
        let config = watch::channel(initial_config);
        let hyprland = watch::channel(HyprlandState::new(Vec::new(), Vec::new()));
        let time = watch::channel(chrono::Local::now());
        let dbus = watch::channel(DBusState::default());
        let applets = watch::channel(AppletsState::default());
        let metrics = watch::channel(crate::domain::metrics::MetricsState::default());

        Self {
            config,
            hyprland,
            time,
            dbus,
            applets,
            metrics,
        }
    }

    pub fn config_tx(&self) -> watch::Sender<Config> {
        self.config.0.clone()
    }

    pub fn config_rx(&self) -> watch::Receiver<Config> {
        self.config.1.clone()
    }

    pub fn hyprland_tx(&self) -> watch::Sender<HyprlandState> {
        self.hyprland.0.clone()
    }

    pub fn hyprland_rx(&self) -> watch::Receiver<HyprlandState> {
        self.hyprland.1.clone()
    }

    pub fn time_tx(&self) -> watch::Sender<chrono::DateTime<chrono::Local>> {
        self.time.0.clone()
    }

    pub fn time_rx(&self) -> watch::Receiver<chrono::DateTime<chrono::Local>> {
        self.time.1.clone()
    }

    pub fn dbus_tx(&self) -> watch::Sender<DBusState> {
        self.dbus.0.clone()
    }

    pub fn dbus_rx(&self) -> watch::Receiver<DBusState> {
        self.dbus.1.clone()
    }

    pub fn applets_tx(&self) -> watch::Sender<AppletsState> {
        self.applets.0.clone()
    }

    pub fn applets_rx(&self) -> watch::Receiver<AppletsState> {
        self.applets.1.clone()
    }

    pub fn metrics_tx(&self) -> watch::Sender<crate::domain::metrics::MetricsState> {
        self.metrics.0.clone()
    }

    pub fn metrics_rx(&self) -> watch::Receiver<crate::domain::metrics::MetricsState> {
        self.metrics.1.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::config::Config;

    #[tokio::test]
    async fn test_signal_hub_config_propagation() {
        let hub = SignalHub::new(Config::default());
        let config_rx = hub.config_rx();
        let config_tx = hub.config_tx();

        let new_config = Config::default();
        config_tx.send(new_config).unwrap();
        
        assert!(config_rx.has_changed().unwrap());
    }

    #[tokio::test]
    async fn test_signal_hub_hyprland_propagation() {
        let hub = SignalHub::new(Config::default());
        let hypr_rx = hub.hyprland_rx();
        let hypr_tx = hub.hyprland_tx();

        let new_state = HyprlandState::new(Vec::new(), Vec::new());
        hypr_tx.send(new_state).unwrap();

        assert!(hypr_rx.has_changed().unwrap());
    }

    #[tokio::test]
    async fn test_signal_hub_time_propagation() {
        let hub = SignalHub::new(Config::default());
        let time_rx = hub.time_rx();
        let time_tx = hub.time_tx();

        let now = chrono::Local::now();
        time_tx.send(now).unwrap();

        assert!(time_rx.has_changed().unwrap());
    }
}
