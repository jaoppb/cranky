use crate::features::workspaces::domain::{Monitor, Workspace};
use crate::shared::config::domain::Config;

use tokio::sync::watch;

use crate::features::systray::domain::SystrayState;
use crate::shared::dbus::domain::{DBusState, DBusSubscription};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SignalKind {
    Time,
    Hyprland,
    DBus(DBusSubscription),
    Systray,
    Metrics,
    Mpris,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HyprlandState {
    workspaces:
        std::collections::BTreeMap<crate::features::workspaces::domain::WorkspaceId, Workspace>,
    monitors: std::collections::BTreeMap<crate::features::workspaces::domain::MonitorName, Monitor>,
    focused_monitor: Option<crate::features::workspaces::domain::MonitorName>,
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
        workspaces: std::collections::BTreeMap<
            crate::features::workspaces::domain::WorkspaceId,
            Workspace,
        >,
        monitors: std::collections::BTreeMap<
            crate::features::workspaces::domain::MonitorName,
            Monitor,
        >,
        focused_monitor: Option<crate::features::workspaces::domain::MonitorName>,
    ) -> Self {
        Self {
            workspaces,
            monitors,
            focused_monitor,
        }
    }

    pub fn monitors(
        &self,
    ) -> &std::collections::BTreeMap<crate::features::workspaces::domain::MonitorName, Monitor>
    {
        &self.monitors
    }

    pub fn workspaces(
        &self,
    ) -> &std::collections::BTreeMap<crate::features::workspaces::domain::WorkspaceId, Workspace>
    {
        &self.workspaces
    }

    pub fn focused_monitor(&self) -> Option<&crate::features::workspaces::domain::MonitorName> {
        self.focused_monitor.as_ref()
    }

    pub fn apply_event(&mut self, event: &crate::shared::events::core::WindowManagerEvent) {
        use crate::features::workspaces::domain::Workspace;
        use crate::shared::events::core::WindowManagerEvent;

        match event {
            WindowManagerEvent::WorkspaceActivated { id, name } => {
                let mon = self.focused_monitor.clone();
                if !self.workspaces.contains_key(id) {
                    // Do not assume the monitor, leave it as None so inconsistency check catches it
                    self.workspaces
                        .insert(id.clone(), Workspace::new(id.clone(), name.clone(), None));
                }

                if let Some(mon_name) = mon
                    && let Some(m) = self.monitors.get_mut(&mon_name)
                {
                    m.set_active_workspace(id.clone());
                }
            }
            WindowManagerEvent::WorkspaceCreated { id, name } => {
                if !self.workspaces.contains_key(id) {
                    self.workspaces
                        .insert(id.clone(), Workspace::new(id.clone(), name.clone(), None));
                }
            }
            WindowManagerEvent::WorkspaceDestroyed { id, name: _ } => {
                self.workspaces.remove(id);
            }
            WindowManagerEvent::WorkspaceMoved {
                id,
                name,
                monitor_name,
            } => {
                if let Some(ws) = self.workspaces.get_mut(id) {
                    ws.set_monitor(monitor_name.clone());
                } else {
                    self.workspaces.insert(
                        id.clone(),
                        Workspace::new(id.clone(), name.clone(), Some(monitor_name.clone())),
                    );
                }
            }
            WindowManagerEvent::WorkspaceRenamed { id, new_name } => {
                if let Some(old) = self.workspaces.get(id) {
                    let new_ws =
                        Workspace::new(id.clone(), new_name.clone(), old.monitor().cloned());
                    self.workspaces.insert(id.clone(), new_ws);
                }
            }
            WindowManagerEvent::MonitorFocused {
                monitor_name,
                workspace_id,
            } => {
                self.focused_monitor = Some(monitor_name.clone());
                if let Some(m) = self.monitors.get_mut(monitor_name) {
                    m.set_active_workspace(workspace_id.clone());
                }

                // Edge case: update workspace's monitor if missing or wrong
                if let Some(ws) = self.workspaces.get_mut(workspace_id) {
                    ws.set_monitor(monitor_name.clone());
                }
            }
            WindowManagerEvent::SpecialWorkspaceActivated {
                id,
                name,
                monitor_name,
            } => {
                if let Some(m) = self.monitors.get_mut(monitor_name) {
                    m.set_special_workspace(id.clone());
                }
                if let Some(ws_id) = id {
                    if let Some(ws) = self.workspaces.get_mut(ws_id) {
                        ws.set_monitor(monitor_name.clone());
                    } else if let Some(ws_name) = name {
                        self.workspaces.insert(
                            ws_id.clone(),
                            Workspace::new(
                                ws_id.clone(),
                                ws_name.clone(),
                                Some(monitor_name.clone()),
                            ),
                        );
                    }
                }
            }
            WindowManagerEvent::ActiveWindowChanged { .. } => {}
            WindowManagerEvent::WindowTitleChanged { .. } => {}
        }
    }
}

pub type ModuleSizesMap =
    std::collections::HashMap<crate::shared::primitives::MonitorId, crate::shared::primitives::ChildSizesMap>;

pub struct SignalHub {
    config: (watch::Sender<Config>, watch::Receiver<Config>),
    hyprland: (watch::Sender<HyprlandState>, watch::Receiver<HyprlandState>),
    time: (
        watch::Sender<chrono::DateTime<chrono::Local>>,
        watch::Receiver<chrono::DateTime<chrono::Local>>,
    ),
    dbus: (watch::Sender<DBusState>, watch::Receiver<DBusState>),
    systray: (watch::Sender<SystrayState>, watch::Receiver<SystrayState>),
    metrics: (
        watch::Sender<crate::features::metrics::domain::MetricsState>,
        watch::Receiver<crate::features::metrics::domain::MetricsState>,
    ),
    pointer: (
        crate::shared::events::core::PointerSender,
        crate::shared::events::core::PointerReceiver,
    ),
    mpris: (
        watch::Sender<crate::features::mpris::domain::MprisState>,
        watch::Receiver<crate::features::mpris::domain::MprisState>,
    ),
    module_sizes: (watch::Sender<ModuleSizesMap>, watch::Receiver<ModuleSizesMap>),
}

impl SignalHub {
    pub fn new(initial_config: Config) -> Self {
        let config = watch::channel(initial_config);
        let hyprland = watch::channel(HyprlandState::new(
            std::collections::BTreeMap::new(),
            std::collections::BTreeMap::new(),
            None,
        ));
        let time = watch::channel(chrono::Local::now());
        let dbus = watch::channel(DBusState::default());
        let systray = watch::channel(SystrayState::default());
        let metrics = watch::channel(crate::features::metrics::domain::MetricsState::default());
        let mpris = watch::channel(crate::features::mpris::domain::MprisState::default());
        let pointer = tokio::sync::broadcast::channel(32);
        let module_sizes = watch::channel(std::collections::HashMap::new());

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
        }
    }

    pub fn module_sizes_tx(&self) -> watch::Sender<ModuleSizesMap> {
        self.module_sizes.0.clone()
    }

    pub fn module_sizes_rx(&self) -> watch::Receiver<ModuleSizesMap> {
        self.module_sizes.1.clone()
    }

    pub fn mpris_tx(&self) -> watch::Sender<crate::features::mpris::domain::MprisState> {
        self.mpris.0.clone()
    }

    pub fn mpris_rx(&self) -> watch::Receiver<crate::features::mpris::domain::MprisState> {
        self.mpris.1.clone()
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

    pub fn systray_tx(&self) -> watch::Sender<SystrayState> {
        self.systray.0.clone()
    }

    pub fn systray_rx(&self) -> watch::Receiver<SystrayState> {
        self.systray.1.clone()
    }

    pub fn metrics_tx(&self) -> watch::Sender<crate::features::metrics::domain::MetricsState> {
        self.metrics.0.clone()
    }

    pub fn metrics_rx(&self) -> watch::Receiver<crate::features::metrics::domain::MetricsState> {
        self.metrics.1.clone()
    }
    pub fn pointer_tx(&self) -> &crate::shared::events::core::PointerSender {
        &self.pointer.0
    }

    pub fn pointer_rx(&self) -> crate::shared::events::core::PointerReceiver {
        self.pointer.0.subscribe()
    }

    pub fn subscribe_streams(
        &self,
        subs: &[SignalKind],
    ) -> futures_util::stream::SelectAll<futures_util::stream::BoxStream<'static, SignalKind>> {
        use futures_util::StreamExt;
        use tokio_stream::wrappers::WatchStream;

        let mut events_stream = futures_util::stream::SelectAll::new();

        if subs.contains(&SignalKind::Time) {
            events_stream.push(
                WatchStream::new(self.time_rx())
                    .map(|_| SignalKind::Time)
                    .boxed(),
            );
        }
        if subs.contains(&SignalKind::Hyprland) {
            events_stream.push(
                WatchStream::new(self.hyprland_rx())
                    .map(|_| SignalKind::Hyprland)
                    .boxed(),
            );
        }
        if subs.contains(&SignalKind::Systray) {
            events_stream.push(
                WatchStream::new(self.systray_rx())
                    .map(|_| SignalKind::Systray)
                    .boxed(),
            );
        }
        if subs.contains(&SignalKind::Metrics) {
            events_stream.push(
                WatchStream::new(self.metrics_rx())
                    .map(|_| SignalKind::Metrics)
                    .boxed(),
            );
        }
        if subs.contains(&SignalKind::Mpris) {
            events_stream.push(
                WatchStream::new(self.mpris_rx())
                    .map(|_| SignalKind::Mpris)
                    .boxed(),
            );
        }
        if subs.iter().any(|s| matches!(s, SignalKind::DBus(_))) {
            events_stream.push(
                WatchStream::new(self.dbus_rx())
                    .map(|_| {
                        SignalKind::DBus(crate::shared::dbus::domain::DBusSubscription::new(
                            crate::shared::dbus::domain::BusType::Session,
                            None,
                            None,
                            None,
                            None,
                        ))
                    })
                    .boxed(),
            );
        }

        events_stream
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::config::domain::Config;

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

        let new_state = HyprlandState::new(
            std::collections::BTreeMap::new(),
            std::collections::BTreeMap::new(),
            None,
        );
        hypr_tx.send(new_state).unwrap();

        assert!(hypr_rx.has_changed().unwrap());
    }

    #[tokio::test]
    async fn test_signal_hub_time_propagation() {
        let hub = SignalHub::new(Config::default());
        let mut time_rx = hub.time_rx();
        let time_tx = hub.time_tx();

        let now = chrono::Local::now();
        time_tx.send(now).unwrap();

        assert!(time_rx.changed().await.is_ok());
    }

    #[test]
    fn test_hyprland_state_apply_event() {
        use crate::features::workspaces::domain::{
            Monitor, MonitorName, WorkspaceId, WorkspaceName,
        };
        use crate::shared::events::core::WindowManagerEvent;

        let mut state = HyprlandState::new(
            std::collections::BTreeMap::new(),
            std::collections::BTreeMap::new(),
            None,
        );

        // Test WorkspaceCreated
        state.apply_event(&WindowManagerEvent::WorkspaceCreated {
            id: WorkspaceId::new(1),
            name: WorkspaceName::new("1"),
        });
        assert!(state.workspaces().contains_key(&WorkspaceId::new(1)));

        // Test WorkspaceActivated
        state.apply_event(&WindowManagerEvent::WorkspaceActivated {
            id: WorkspaceId::new(2),
            name: WorkspaceName::new("2"),
        });
        assert!(state.workspaces().contains_key(&WorkspaceId::new(2)));

        // Test WorkspaceDestroyed
        state.apply_event(&WindowManagerEvent::WorkspaceDestroyed {
            id: WorkspaceId::new(1),
            name: WorkspaceName::new("1"),
        });
        assert!(!state.workspaces().contains_key(&WorkspaceId::new(1)));

        // Test WorkspaceMoved
        state.apply_event(&WindowManagerEvent::WorkspaceMoved {
            id: WorkspaceId::new(2),
            name: WorkspaceName::new("2"),
            monitor_name: MonitorName::new("DP-1"),
        });
        assert_eq!(
            state
                .workspaces()
                .get(&WorkspaceId::new(2))
                .unwrap()
                .monitor(),
            Some(&MonitorName::new("DP-1"))
        );

        // Test WorkspaceMoved (new workspace)
        state.apply_event(&WindowManagerEvent::WorkspaceMoved {
            id: WorkspaceId::new(3),
            name: WorkspaceName::new("3"),
            monitor_name: MonitorName::new("DP-2"),
        });
        assert_eq!(
            state
                .workspaces()
                .get(&WorkspaceId::new(3))
                .unwrap()
                .monitor(),
            Some(&MonitorName::new("DP-2"))
        );

        // Test WorkspaceRenamed
        state.apply_event(&WindowManagerEvent::WorkspaceRenamed {
            id: WorkspaceId::new(2),
            new_name: WorkspaceName::new("2-renamed"),
        });
        assert!(state.workspaces().contains_key(&WorkspaceId::new(2)));

        // Test MonitorFocused
        state.monitors.insert(
            MonitorName::new("DP-1"),
            Monitor::new(MonitorName::new("DP-1"), WorkspaceId::new(1), None),
        );
        state.apply_event(&WindowManagerEvent::MonitorFocused {
            monitor_name: MonitorName::new("DP-1"),
            workspace_id: WorkspaceId::new(2),
        });
        assert_eq!(state.focused_monitor(), Some(&MonitorName::new("DP-1")));
        assert_eq!(
            state
                .monitors()
                .get(&MonitorName::new("DP-1"))
                .unwrap()
                .active_workspace_id(),
            &WorkspaceId::new(2)
        );

        // Test SpecialWorkspaceActivated
        state.apply_event(&WindowManagerEvent::SpecialWorkspaceActivated {
            id: Some(WorkspaceId::new(99)),
            name: Some(WorkspaceName::new("special")),
            monitor_name: MonitorName::new("DP-1"),
        });
        assert_eq!(
            state
                .monitors()
                .get(&MonitorName::new("DP-1"))
                .unwrap()
                .special_workspace_id(),
            Some(&WorkspaceId::new(99))
        );
        assert_eq!(
            state
                .workspaces()
                .get(&WorkspaceId::new(99))
                .unwrap()
                .monitor(),
            Some(&MonitorName::new("DP-1"))
        );
    }

    #[test]
    fn test_hyprland_state_serialize() {
        let mut workspaces = std::collections::BTreeMap::new();
        workspaces.insert(
            crate::features::workspaces::domain::WorkspaceId::new(1),
            crate::features::workspaces::domain::Workspace::new(
                crate::features::workspaces::domain::WorkspaceId::new(1),
                crate::features::workspaces::domain::WorkspaceName::new("1"),
                None,
            ),
        );
        let state = HyprlandState::new(workspaces, std::collections::BTreeMap::new(), None);

        let json = serde_json::to_string(&state).unwrap();
        assert!(json.contains("\"workspaces\":["));
        assert!(json.contains("\"id\":1"));
    }

    #[tokio::test]
    async fn test_signal_hub_other_propagation() {
        let hub = SignalHub::new(Config::default());

        let dbus_tx = hub.dbus_tx();
        let mut dbus_rx = hub.dbus_rx();
        dbus_tx.send(DBusState::default()).unwrap();
        assert!(dbus_rx.changed().await.is_ok());

        let systray_tx = hub.systray_tx();
        let mut systray_rx = hub.systray_rx();
        systray_tx.send(SystrayState::default()).unwrap();
        assert!(systray_rx.changed().await.is_ok());

        let metrics_tx = hub.metrics_tx();
        let mut metrics_rx = hub.metrics_rx();
        metrics_tx
            .send(crate::features::metrics::domain::MetricsState::default())
            .unwrap();
        assert!(metrics_rx.changed().await.is_ok());

        let ptr_tx = hub.pointer_tx();
        let mut ptr_rx = hub.pointer_rx();
        ptr_tx
            .send((
                crate::shared::primitives::ModuleId::new(1),
                crate::shared::primitives::MonitorId::new("1"),
                crate::shared::events::core::PointerEvent::PointerLeave,
            ))
            .unwrap();
        assert!(ptr_rx.recv().await.is_ok());
    }

    #[tokio::test]
    async fn test_signal_hub_subscribe_streams() {
        use futures_util::StreamExt;

        let hub = SignalHub::new(Config::default());
        let subs = vec![SignalKind::Time, SignalKind::Systray];
        let mut stream = hub.subscribe_streams(&subs);

        // Initial values are yielded by WatchStreams
        let first = stream.next().await;
        assert!(first.is_some());
        let second = stream.next().await;
        assert!(second.is_some());

        // Subsequent trigger yields signal
        hub.time_tx().send(chrono::Local::now()).unwrap();
        let sig = stream.next().await;
        assert_eq!(sig, Some(SignalKind::Time));
    }
}
