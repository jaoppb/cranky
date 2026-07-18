use crate::domain::config::Config;
use crate::domain::workspace::{Monitor, Workspace};

use tokio::sync::watch;

use crate::domain::applets::AppletsState;
use crate::domain::dbus::{DBusState, DBusSubscription};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SignalKind {
    Time,
    Hyprland,
    DBus(DBusSubscription),
    Applets,
    Metrics,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HyprlandState {
    workspaces: std::collections::BTreeMap<crate::domain::workspace::WorkspaceId, Workspace>,
    monitors: std::collections::BTreeMap<crate::domain::workspace::MonitorName, Monitor>,
    focused_monitor: Option<crate::domain::workspace::MonitorName>,
}

impl serde::Serialize for HyprlandState {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("HyprlandState", 3)?;
        
        let workspaces_vec: Vec<_> = self.workspaces.values().collect();
        state.serialize_field("workspaces", &workspaces_vec)?;
        
        let monitors_vec: Vec<_> = self.monitors.values().collect();
        state.serialize_field("monitors", &monitors_vec)?;
        
        state.serialize_field("focused_monitor", &self.focused_monitor)?;
        state.end()
    }
}

impl HyprlandState {
    pub fn new(
        workspaces: std::collections::BTreeMap<crate::domain::workspace::WorkspaceId, Workspace>,
        monitors: std::collections::BTreeMap<crate::domain::workspace::MonitorName, Monitor>,
        focused_monitor: Option<crate::domain::workspace::MonitorName>,
    ) -> Self {
        Self {
            workspaces,
            monitors,
            focused_monitor,
        }
    }
    
    pub fn monitors(&self) -> &std::collections::BTreeMap<crate::domain::workspace::MonitorName, Monitor> {
        &self.monitors
    }
    
    pub fn workspaces(&self) -> &std::collections::BTreeMap<crate::domain::workspace::WorkspaceId, Workspace> {
        &self.workspaces
    }
    
    pub fn focused_monitor(&self) -> Option<&crate::domain::workspace::MonitorName> {
        self.focused_monitor.as_ref()
    }
    
    pub fn apply_event(&mut self, event: &crate::domain::events::WindowManagerEvent) {
        use crate::domain::events::WindowManagerEvent;
        use crate::domain::workspace::Workspace;
        
        match event {
            WindowManagerEvent::WorkspaceActivated { id, name } => {
                let mon = self.focused_monitor.clone();
                if !self.workspaces.contains_key(id) {
                    // Do not assume the monitor, leave it as None so inconsistency check catches it
                    self.workspaces.insert(id.clone(), Workspace::new(id.clone(), name.clone(), None));
                }
                
                if let Some(mon_name) = mon
                    && let Some(m) = self.monitors.get_mut(&mon_name) {
                        m.set_active_workspace(id.clone());
                    }
            }
            WindowManagerEvent::WorkspaceCreated { id, name } => {
                if !self.workspaces.contains_key(id) {
                    self.workspaces.insert(id.clone(), Workspace::new(id.clone(), name.clone(), None));
                }
            }
            WindowManagerEvent::WorkspaceDestroyed { id, name: _ } => {
                self.workspaces.remove(id);
            }
            WindowManagerEvent::WorkspaceMoved { id, name, monitor_name } => {
                if let Some(ws) = self.workspaces.get_mut(id) {
                    ws.set_monitor(monitor_name.clone());
                } else {
                    self.workspaces.insert(id.clone(), Workspace::new(id.clone(), name.clone(), Some(monitor_name.clone())));
                }
            }
            WindowManagerEvent::WorkspaceRenamed { id, new_name } => {
                if let Some(old) = self.workspaces.get(id) {
                    let new_ws = Workspace::new(id.clone(), new_name.clone(), old.monitor().cloned());
                    self.workspaces.insert(id.clone(), new_ws);
                }
            }
            WindowManagerEvent::MonitorFocused { monitor_name, workspace_id } => {
                self.focused_monitor = Some(monitor_name.clone());
                if let Some(m) = self.monitors.get_mut(monitor_name) {
                    m.set_active_workspace(workspace_id.clone());
                }
                
                // Edge case: update workspace's monitor if missing or wrong
                if let Some(ws) = self.workspaces.get_mut(workspace_id) {
                    ws.set_monitor(monitor_name.clone());
                }
            }
            WindowManagerEvent::SpecialWorkspaceActivated { id, name: _, monitor_name } => {
                if let Some(m) = self.monitors.get_mut(monitor_name) {
                    m.set_special_workspace(id.clone());
                }
            }
            WindowManagerEvent::ActiveWindowChanged { .. } => {}
            WindowManagerEvent::WindowTitleChanged { .. } => {}
        }
    }
}

pub struct SignalHub {
    config: (watch::Sender<Config>, watch::Receiver<Config>),
    hyprland: (watch::Sender<HyprlandState>, watch::Receiver<HyprlandState>),
    time: (
        watch::Sender<chrono::DateTime<chrono::Local>>,
        watch::Receiver<chrono::DateTime<chrono::Local>>,
    ),
    dbus: (watch::Sender<DBusState>, watch::Receiver<DBusState>),
    applets: (watch::Sender<AppletsState>, watch::Receiver<AppletsState>),
    metrics: (
        watch::Sender<crate::domain::metrics::MetricsState>,
        watch::Receiver<crate::domain::metrics::MetricsState>,
    ),
    pointer: (
        crate::domain::events::PointerSender,
        crate::domain::events::PointerReceiver,
    ),
}

impl SignalHub {
    pub fn new(initial_config: Config) -> Self {
        let config = watch::channel(initial_config);
        let hyprland = watch::channel(HyprlandState::new(std::collections::BTreeMap::new(), std::collections::BTreeMap::new(), None));
        let time = watch::channel(chrono::Local::now());
        let dbus = watch::channel(DBusState::default());
        let applets = watch::channel(AppletsState::default());
        let metrics = watch::channel(crate::domain::metrics::MetricsState::default());
        let pointer = tokio::sync::broadcast::channel(32);

        Self {
            config,
            hyprland,
            time,
            dbus,
            applets,
            metrics,
            pointer,
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
    pub fn pointer_tx(
        &self,
    ) -> &crate::domain::events::PointerSender {
        &self.pointer.0
    }

    pub fn pointer_rx(
        &self,
    ) -> crate::domain::events::PointerReceiver {
        self.pointer.0.subscribe()
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

        let new_state = HyprlandState::new(std::collections::BTreeMap::new(), std::collections::BTreeMap::new(), None);
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
