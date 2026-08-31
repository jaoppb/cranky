use crate::features::workspaces::adapters::hyprland_provider::{
    HyprlandProvider, RealHyprlandProvider,
};
use crate::features::workspaces::domain::{
    Monitor, MonitorName, Workspace, WorkspaceId, WorkspaceName,
};
use crate::features::workspaces::ports::WindowManagerError;
use crate::features::workspaces::ports::WindowManagerPort;
use crate::shared::events::signals::{HyprlandState, SignalHub};
use serde::{Deserialize, Serialize};
use std::io::{BufRead, ErrorKind};
use std::sync::Arc;

/// Represents a specific consistency failure in the internal Hyprland state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StateInconsistency {
    /// A workspace exists in state but has no assigned monitor.
    WorkspaceMissingMonitor {
        workspace_id: WorkspaceId,
        workspace_name: WorkspaceName,
    },
    /// A monitor references an active workspace that does not exist in the state.
    MonitorActiveWorkspaceNotFound {
        monitor_name: MonitorName,
        workspace_id: WorkspaceId,
    },
    /// A monitor references an active workspace whose assigned monitor differs from the monitor itself.
    MonitorActiveWorkspaceMismatch {
        monitor_name: MonitorName,
        workspace_id: WorkspaceId,
        workspace_monitor: Option<MonitorName>,
    },
    /// A monitor references a special workspace that does not exist in the state.
    MonitorSpecialWorkspaceNotFound {
        monitor_name: MonitorName,
        special_workspace_id: WorkspaceId,
    },
    /// A monitor references a special workspace whose assigned monitor differs from the monitor itself.
    MonitorSpecialWorkspaceMismatch {
        monitor_name: MonitorName,
        special_workspace_id: WorkspaceId,
        workspace_monitor: Option<MonitorName>,
    },
}

impl std::fmt::Display for StateInconsistency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WorkspaceMissingMonitor {
                workspace_id,
                workspace_name,
            } => {
                write!(
                    f,
                    "workspace {workspace_id} ('{workspace_name}') has no monitor assigned"
                )
            }
            Self::MonitorActiveWorkspaceNotFound {
                monitor_name,
                workspace_id,
            } => {
                write!(
                    f,
                    "monitor '{monitor_name}' active workspace {workspace_id} is not present in known workspaces"
                )
            }
            Self::MonitorActiveWorkspaceMismatch {
                monitor_name,
                workspace_id,
                workspace_monitor,
            } => {
                let actual = workspace_monitor
                    .as_ref()
                    .map_or("None", MonitorName::as_str);
                write!(
                    f,
                    "monitor '{monitor_name}' active workspace {workspace_id} is assigned to monitor '{actual}' instead of '{monitor_name}'"
                )
            }
            Self::MonitorSpecialWorkspaceNotFound {
                monitor_name,
                special_workspace_id,
            } => {
                write!(
                    f,
                    "monitor '{monitor_name}' special workspace {special_workspace_id} is not present in known workspaces"
                )
            }
            Self::MonitorSpecialWorkspaceMismatch {
                monitor_name,
                special_workspace_id,
                workspace_monitor,
            } => {
                let actual = workspace_monitor
                    .as_ref()
                    .map_or("None", MonitorName::as_str);
                write!(
                    f,
                    "monitor '{monitor_name}' special workspace {special_workspace_id} is assigned to monitor '{actual}' instead of '{monitor_name}'"
                )
            }
        }
    }
}

#[derive(Deserialize)]
struct HyprWorkspaceDto {
    id: i32,
    name: String,
    monitor: String,
}

impl HyprWorkspaceDto {
    #[must_use]
    fn into_domain(self) -> Workspace {
        Workspace::new(
            crate::features::workspaces::domain::WorkspaceId::new(self.id),
            crate::features::workspaces::domain::WorkspaceName::new(self.name),
            Some(crate::features::workspaces::domain::MonitorName::new(
                self.monitor,
            )),
        )
    }
}

#[derive(Deserialize)]
struct HyprMonitorDto {
    name: String,
    #[serde(rename = "activeWorkspace")]
    active_workspace: HyprActiveWorkspaceDto,
    #[serde(rename = "specialWorkspace")]
    special_workspace: HyprActiveWorkspaceDto,
    focused: bool,
}

#[derive(Deserialize)]
struct HyprActiveWorkspaceDto {
    id: i32,
}

impl HyprMonitorDto {
    #[must_use]
    fn into_domain(self) -> (Monitor, bool) {
        let special_ws_id = if self.special_workspace.id != 0 {
            Some(crate::features::workspaces::domain::WorkspaceId::new(
                self.special_workspace.id,
            ))
        } else {
            None
        };
        let monitor = Monitor::new(
            crate::features::workspaces::domain::MonitorName::new(self.name),
            crate::features::workspaces::domain::WorkspaceId::new(self.active_workspace.id),
            special_ws_id,
        );
        (monitor, self.focused)
    }
}

pub struct HyprlandAdapter {
    provider: Arc<dyn HyprlandProvider>,
}

impl HyprlandAdapter {
    #[must_use]
    pub fn new(app_env: std::sync::Arc<crate::shared::env::domain::AppEnvironment>) -> Self {
        Self {
            provider: Arc::new(RealHyprlandProvider::new(app_env)),
        }
    }

    #[cfg(test)]
    #[must_use]
    pub fn with_provider(provider: Arc<dyn HyprlandProvider>) -> Self {
        Self { provider }
    }

    fn parse_event(raw_line: &str) -> Option<crate::shared::events::core::WindowManagerEvent> {
        use crate::features::workspaces::domain::{MonitorName, WorkspaceId, WorkspaceName};
        use crate::shared::events::core::{WindowAddress, WindowManagerEvent, WindowTitle};

        let (event_type, payload) = raw_line.trim().split_once(">>")?;

        match event_type {
            "workspacev2" => {
                let (id_str, name) = payload.split_once(',')?;
                Some(WindowManagerEvent::WorkspaceActivated {
                    id: WorkspaceId::new(id_str.parse().ok()?),
                    name: WorkspaceName::new(name),
                })
            }
            "focusedmonv2" => {
                let (mon_name, id_str) = payload.split_once(',')?;
                Some(WindowManagerEvent::MonitorFocused {
                    monitor_name: MonitorName::new(mon_name),
                    workspace_id: WorkspaceId::new(id_str.parse().ok()?),
                })
            }
            "createworkspacev2" => {
                let (id_str, name) = payload.split_once(',')?;
                Some(WindowManagerEvent::WorkspaceCreated {
                    id: WorkspaceId::new(id_str.parse().ok()?),
                    name: WorkspaceName::new(name),
                })
            }
            "destroyworkspacev2" => {
                let (id_str, name) = payload.split_once(',')?;
                Some(WindowManagerEvent::WorkspaceDestroyed {
                    id: WorkspaceId::new(id_str.parse().ok()?),
                    name: WorkspaceName::new(name),
                })
            }
            "moveworkspacev2" => {
                let mut parts = payload.splitn(3, ',');
                let id_str = parts.next()?;
                let name = parts.next()?;
                let monitor_name = parts.next()?;
                Some(WindowManagerEvent::WorkspaceMoved {
                    id: WorkspaceId::new(id_str.parse().ok()?),
                    name: WorkspaceName::new(name),
                    monitor_name: MonitorName::new(monitor_name),
                })
            }
            "renameworkspace" => {
                let (id_str, name) = payload.split_once(',')?;
                Some(WindowManagerEvent::WorkspaceRenamed {
                    id: WorkspaceId::new(id_str.parse().ok()?),
                    new_name: WorkspaceName::new(name),
                })
            }
            "activespecialv2" => {
                let mut parts = payload.splitn(3, ',');
                let id_str = parts.next()?;
                let name_str = parts.next()?;
                let monitor_str = parts.next()?;
                let id = match id_str.parse::<i32>() {
                    Ok(0) | Err(_) => None,
                    Ok(val) => Some(WorkspaceId::new(val)),
                };
                let ws_name = if id.is_none() || name_str.is_empty() {
                    None
                } else {
                    Some(WorkspaceName::new(name_str))
                };
                Some(WindowManagerEvent::SpecialWorkspaceActivated {
                    id,
                    name: ws_name,
                    monitor_name: MonitorName::new(monitor_str),
                })
            }
            "activewindowv2" => Some(WindowManagerEvent::ActiveWindowChanged {
                address: WindowAddress::new(payload),
            }),
            "windowtitlev2" => {
                let (address, title) = payload.split_once(',')?;
                Some(WindowManagerEvent::WindowTitleChanged {
                    address: WindowAddress::new(address),
                    title: WindowTitle::new(title),
                })
            }
            _ => None,
        }
    }

    /// Checks for inconsistencies in the current Hyprland state.
    ///
    /// Returns a list of structured [`StateInconsistency`] items found.
    #[must_use]
    pub fn find_state_inconsistencies(state: &HyprlandState) -> Vec<StateInconsistency> {
        let mut inconsistencies = Vec::new();

        for (ws_id, ws) in state.workspaces() {
            if ws.monitor().is_none() {
                inconsistencies.push(StateInconsistency::WorkspaceMissingMonitor {
                    workspace_id: ws_id.clone(),
                    workspace_name: ws.name().clone(),
                });
            }
        }

        for (mon_name, mon) in state.monitors() {
            if let Some(ws) = state.workspaces().get(mon.active_workspace_id()) {
                if ws.monitor() != Some(mon.name()) {
                    inconsistencies.push(StateInconsistency::MonitorActiveWorkspaceMismatch {
                        monitor_name: mon_name.clone(),
                        workspace_id: mon.active_workspace_id().clone(),
                        workspace_monitor: ws.monitor().cloned(),
                    });
                }
            } else {
                inconsistencies.push(StateInconsistency::MonitorActiveWorkspaceNotFound {
                    monitor_name: mon_name.clone(),
                    workspace_id: mon.active_workspace_id().clone(),
                });
            }

            if let Some(sp_id) = mon.special_workspace_id() {
                if let Some(ws) = state.workspaces().get(sp_id) {
                    if ws.monitor() != Some(mon.name()) {
                        inconsistencies.push(StateInconsistency::MonitorSpecialWorkspaceMismatch {
                            monitor_name: mon_name.clone(),
                            special_workspace_id: sp_id.clone(),
                            workspace_monitor: ws.monitor().cloned(),
                        });
                    }
                } else {
                    inconsistencies.push(StateInconsistency::MonitorSpecialWorkspaceNotFound {
                        monitor_name: mon_name.clone(),
                        special_workspace_id: sp_id.clone(),
                    });
                }
            }
        }

        inconsistencies
    }

    /// Runs a background loop that listens to Hyprland event socket and pushes updates to the `SignalHub`.
    #[allow(
        clippy::unused_async,
        clippy::too_many_lines,
        clippy::useless_let_if_seq
    )]
    pub async fn run(self, hub: Arc<SignalHub>) {
        // Since we are reading from a socket using blocking std::io we can't easily use tokio::time::timeout
        // directly on the reader without wrapping it. We will use a channel to bridge to async, or just
        // use tokio::net::UnixStream. Wait, `provider.listen_events()` returns `std::os::unix::net::UnixStream`.
        // Let's set it to non-blocking and bridge it to tokio.
        tokio::task::spawn_blocking(move || {
            let hypr_tx = hub.hyprland_tx();

            loop {
                let stream = match self.provider.listen_events() {
                    Ok(s) => s,
                    Err(e) => {
                        tracing::error!("Failed to connect to Hyprland event socket: {e}");
                        std::thread::sleep(std::time::Duration::from_secs(1));
                        continue;
                    }
                };

                // set read timeout to simulate batching drain (2ms)
                let _ = stream.set_read_timeout(Some(std::time::Duration::from_millis(2)));

                let mut reader = std::io::BufReader::new(stream);

                // Initial fetch
                let mut current_state = match self.get_state() {
                    Ok((workspaces, monitors, focused)) => {
                        let state = HyprlandState::new(workspaces, monitors, focused);
                        if *hypr_tx.borrow() != state {
                            let _ = hypr_tx.send(state.clone());
                        }
                        state
                    }
                    Err(e) => {
                        tracing::error!("Hyprland adapter error on initial fetch: {e}");
                        HyprlandState::new(
                            std::collections::BTreeMap::new(),
                            std::collections::BTreeMap::new(),
                            None,
                        )
                    }
                };

                let mut line = String::new();
                loop {
                    line.clear();
                    // We must wait for the FIRST event in a batch.
                    // Because we set a 2ms read timeout on the socket, it will frequently timeout if idle.
                    // If it times out, we just loop again (acting as a polling loop but it's okay for now,
                    // wait, polling is bad! We should restore infinite timeout for the first event).
                    let _ = reader.get_mut().set_read_timeout(None);

                    match reader.read_line(&mut line) {
                        Ok(0) => {
                            tracing::info!("Hyprland event socket closed, reconnecting...");
                            break;
                        }
                        Ok(_) => {
                            let mut batch_events = Vec::new();
                            let trimmed = line.trim();
                            if !trimmed.is_empty() {
                                tracing::trace!(event = trimmed, "Hyprland event received");
                                batch_events.push(trimmed.to_string());
                            }

                            let mut state_changed = false;
                            if let Some(event) = Self::parse_event(&line) {
                                current_state.apply_event(&event);
                                state_changed = true;
                            }

                            // Now, quickly drain any other immediately pending events (batching)
                            let _ = reader
                                .get_mut()
                                .set_read_timeout(Some(std::time::Duration::from_millis(2)));
                            loop {
                                line.clear();
                                match reader.read_line(&mut line) {
                                    Ok(0) => break, // EOF
                                    Ok(_) => {
                                        let trimmed = line.trim();
                                        if !trimmed.is_empty() {
                                            tracing::trace!(event = trimmed, "Hyprland event received");
                                            batch_events.push(trimmed.to_string());
                                        }
                                        if let Some(event) = Self::parse_event(&line) {
                                            current_state.apply_event(&event);
                                            state_changed = true;
                                        }
                                    }
                                    Err(e)
                                        if e.kind() == ErrorKind::WouldBlock
                                            || e.kind() == ErrorKind::TimedOut =>
                                    {
                                        break; // End of burst
                                    }
                                    Err(_) => break, // Other error
                                }
                            }

                            // Validate state consistency before broadcast
                            if state_changed {
                                let inconsistencies =
                                    Self::find_state_inconsistencies(&current_state);

                                if !inconsistencies.is_empty() {
                                    tracing::warn!(
                                        reasons = ?inconsistencies,
                                        batch_events = ?batch_events,
                                        "Hyprland state inconsistent after event batch, forcing full resync"
                                    );
                                    match self.get_state() {
                                        Ok((workspaces, monitors, focused)) => {
                                            current_state =
                                                HyprlandState::new(workspaces, monitors, focused);
                                        }
                                        Err(e) => {
                                            tracing::error!(
                                                "Failed to resync Hyprland state after inconsistency: {e}"
                                            );
                                        }
                                    }
                                }
                            }

                            // Broadcast reduced state
                            if state_changed && *hypr_tx.borrow() != current_state {
                                let _ = hypr_tx.send(current_state.clone());
                            }
                        }
                        Err(e) => {
                            tracing::error!("Hyprland event socket read error: {e}");
                            break;
                        }
                    }
                }

                std::thread::sleep(std::time::Duration::from_secs(1));
            }
        });
    }
}

impl WindowManagerPort for HyprlandAdapter {
    #[tracing::instrument(skip(self), err)]
    fn get_state(
        &self,
    ) -> Result<crate::features::workspaces::ports::WindowManagerState, WindowManagerError> {
        let ws_json =
            self.provider
                .query_workspaces()
                .map_err(|e| WindowManagerError::IpcError {
                    reason: format!("Failed to get workspaces: {e}"),
                })?;
        let mon_json =
            self.provider
                .query_monitors()
                .map_err(|e| WindowManagerError::IpcError {
                    reason: format!("Failed to get monitors: {e}"),
                })?;

        let workspaces: std::collections::BTreeMap<_, _> =
            serde_json::from_str::<Vec<HyprWorkspaceDto>>(&ws_json)
                .map_err(|e| WindowManagerError::IpcError {
                    reason: e.to_string(),
                })?
                .into_iter()
                .map(|dto| {
                    let ws = dto.into_domain();
                    (ws.id().clone(), ws)
                })
                .collect();

        let mut focused_monitor = None;
        let monitors: std::collections::BTreeMap<_, _> =
            serde_json::from_str::<Vec<HyprMonitorDto>>(&mon_json)
                .map_err(|e| WindowManagerError::IpcError {
                    reason: e.to_string(),
                })?
                .into_iter()
                .map(|dto| {
                    let (mon, is_focused) = dto.into_domain();
                    if is_focused {
                        focused_monitor = Some(mon.name().clone());
                    }
                    (mon.name().clone(), mon)
                })
                .collect();

        Ok((workspaces, monitors, focused_monitor))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::workspaces::adapters::hyprland_provider::MockHyprlandProvider;

    #[tokio::test]
    async fn test_hyprland_adapter_get_state() {
        let mut mock_provider = MockHyprlandProvider::new();
        mock_provider
            .expect_query_workspaces()
            .times(1)
            .returning(|| Ok("[]".to_string()));
        mock_provider
            .expect_query_monitors()
            .times(1)
            .returning(|| Ok("[]".to_string()));

        let adapter = HyprlandAdapter::with_provider(Arc::new(mock_provider));

        let res = adapter.get_state().unwrap();
        assert_eq!(res.0.len(), 0);
        assert_eq!(res.1.len(), 0);
        assert_eq!(res.2, None);
    }

    #[tokio::test]
    async fn test_hyprland_adapter_get_state_valid() {
        let mut mock_provider = MockHyprlandProvider::new();
        mock_provider
            .expect_query_workspaces()
            .times(1)
            .returning(|| Ok(r#"[{"id": 1, "name": "1", "monitor": "DP-1"}]"#.to_string()));
        mock_provider
            .expect_query_monitors()
            .times(1)
            .returning(|| Ok(r#"[{"name": "DP-1", "activeWorkspace": {"id": 1}, "specialWorkspace": {"id": 0}, "focused": true}]"#.to_string()));

        let adapter = HyprlandAdapter::with_provider(Arc::new(mock_provider));

        let res = adapter.get_state().unwrap();
        assert_eq!(res.0.len(), 1); // workspaces
        assert_eq!(res.1.len(), 1); // monitors
        assert_eq!(
            res.2,
            Some(crate::features::workspaces::domain::MonitorName::new(
                "DP-1"
            ))
        ); // focused
    }

    #[test]
    fn test_parse_event() {
        use crate::features::workspaces::domain::{MonitorName, WorkspaceId, WorkspaceName};
        use crate::shared::events::core::{WindowAddress, WindowManagerEvent, WindowTitle};

        // workspacev2
        let e = HyprlandAdapter::parse_event("workspacev2>>1,test_ws").unwrap();
        assert_eq!(
            e,
            WindowManagerEvent::WorkspaceActivated {
                id: WorkspaceId::new(1),
                name: WorkspaceName::new("test_ws"),
            }
        );

        // focusedmonv2
        let e = HyprlandAdapter::parse_event("focusedmonv2>>DP-1,2").unwrap();
        assert_eq!(
            e,
            WindowManagerEvent::MonitorFocused {
                monitor_name: MonitorName::new("DP-1"),
                workspace_id: WorkspaceId::new(2),
            }
        );

        // createworkspacev2
        let e = HyprlandAdapter::parse_event("createworkspacev2>>3,new_ws").unwrap();
        assert_eq!(
            e,
            WindowManagerEvent::WorkspaceCreated {
                id: WorkspaceId::new(3),
                name: WorkspaceName::new("new_ws"),
            }
        );

        // destroyworkspacev2
        let e = HyprlandAdapter::parse_event("destroyworkspacev2>>4,old_ws").unwrap();
        assert_eq!(
            e,
            WindowManagerEvent::WorkspaceDestroyed {
                id: WorkspaceId::new(4),
                name: WorkspaceName::new("old_ws"),
            }
        );

        // moveworkspacev2
        let e = HyprlandAdapter::parse_event("moveworkspacev2>>5,moved_ws,HDMI-1").unwrap();
        assert_eq!(
            e,
            WindowManagerEvent::WorkspaceMoved {
                id: WorkspaceId::new(5),
                name: WorkspaceName::new("moved_ws"),
                monitor_name: MonitorName::new("HDMI-1"),
            }
        );

        // renameworkspace
        let e = HyprlandAdapter::parse_event("renameworkspace>>6,renamed_ws").unwrap();
        assert_eq!(
            e,
            WindowManagerEvent::WorkspaceRenamed {
                id: WorkspaceId::new(6),
                new_name: WorkspaceName::new("renamed_ws"),
            }
        );

        // activespecialv2
        let e = HyprlandAdapter::parse_event("activespecialv2>>7,special,DP-2").unwrap();
        assert_eq!(
            e,
            WindowManagerEvent::SpecialWorkspaceActivated {
                id: Some(WorkspaceId::new(7)),
                name: Some(WorkspaceName::new("special")),
                monitor_name: MonitorName::new("DP-2"),
            }
        );

        // activespecialv2 closed (0 ID)
        let e = HyprlandAdapter::parse_event("activespecialv2>>0,,DP-2").unwrap();
        assert_eq!(
            e,
            WindowManagerEvent::SpecialWorkspaceActivated {
                id: None,
                name: None,
                monitor_name: MonitorName::new("DP-2"),
            }
        );

        // activespecialv2 negative ID
        let e = HyprlandAdapter::parse_event("activespecialv2>>-98,special:magic,DP-2").unwrap();
        assert_eq!(
            e,
            WindowManagerEvent::SpecialWorkspaceActivated {
                id: Some(WorkspaceId::new(-98)),
                name: Some(WorkspaceName::new("special:magic")),
                monitor_name: MonitorName::new("DP-2"),
            }
        );

        // activewindowv2
        let e = HyprlandAdapter::parse_event("activewindowv2>>0x123").unwrap();
        assert_eq!(
            e,
            WindowManagerEvent::ActiveWindowChanged {
                address: WindowAddress::new("0x123"),
            }
        );

        // windowtitlev2
        let e = HyprlandAdapter::parse_event("windowtitlev2>>0x456,Title").unwrap();
        assert_eq!(
            e,
            WindowManagerEvent::WindowTitleChanged {
                address: WindowAddress::new("0x456"),
                title: WindowTitle::new("Title"),
            }
        );

        // invalid
        assert_eq!(HyprlandAdapter::parse_event("invalid>>data"), None);
        assert_eq!(HyprlandAdapter::parse_event("missing"), None);
    }

    #[tokio::test]
    async fn test_hyprland_adapter_run() {
        use std::io::Write;
        use std::os::unix::net::UnixStream;

        let (mut sender, receiver) = UnixStream::pair().unwrap();

        let mut mock_provider = MockHyprlandProvider::new();
        mock_provider
            .expect_query_workspaces()
            .returning(|| Ok("[]".to_string()));
        mock_provider
            .expect_query_monitors()
            .returning(|| Ok("[]".to_string()));

        mock_provider
            .expect_listen_events()
            .times(1)
            .returning(move || {
                let stream = receiver.try_clone().unwrap();
                Ok(stream)
            });

        let adapter = HyprlandAdapter::with_provider(Arc::new(mock_provider));

        let config = crate::shared::config::domain::Config::default();
        let hub = Arc::new(SignalHub::new(config));

        let run_handle = tokio::task::spawn(adapter.run(hub.clone()));

        sender.write_all(b"workspacev2>>1,test_ws\n").unwrap();

        tokio::time::sleep(std::time::Duration::from_millis(150)).await;

        drop(sender);

        let _ = run_handle.await;
    }

    #[test]
    fn test_state_inconsistency_display() {
        let ws_id = WorkspaceId::new(1);
        let ws_name = WorkspaceName::new("code");
        let mon_dp1 = MonitorName::new("DP-1");
        let mon_hdmi = MonitorName::new("HDMI-1");

        let inc1 = StateInconsistency::WorkspaceMissingMonitor {
            workspace_id: ws_id.clone(),
            workspace_name: ws_name,
        };
        assert_eq!(
            inc1.to_string(),
            "workspace 1 ('code') has no monitor assigned"
        );

        let inc2 = StateInconsistency::MonitorActiveWorkspaceNotFound {
            monitor_name: mon_dp1.clone(),
            workspace_id: ws_id.clone(),
        };
        assert_eq!(
            inc2.to_string(),
            "monitor 'DP-1' active workspace 1 is not present in known workspaces"
        );

        let inc3 = StateInconsistency::MonitorActiveWorkspaceMismatch {
            monitor_name: mon_dp1.clone(),
            workspace_id: ws_id.clone(),
            workspace_monitor: Some(mon_hdmi.clone()),
        };
        assert_eq!(
            inc3.to_string(),
            "monitor 'DP-1' active workspace 1 is assigned to monitor 'HDMI-1' instead of 'DP-1'"
        );

        let inc3_none = StateInconsistency::MonitorActiveWorkspaceMismatch {
            monitor_name: mon_dp1.clone(),
            workspace_id: ws_id.clone(),
            workspace_monitor: None,
        };
        assert_eq!(
            inc3_none.to_string(),
            "monitor 'DP-1' active workspace 1 is assigned to monitor 'None' instead of 'DP-1'"
        );

        let inc4 = StateInconsistency::MonitorSpecialWorkspaceNotFound {
            monitor_name: mon_dp1.clone(),
            special_workspace_id: ws_id.clone(),
        };
        assert_eq!(
            inc4.to_string(),
            "monitor 'DP-1' special workspace 1 is not present in known workspaces"
        );

        let inc5 = StateInconsistency::MonitorSpecialWorkspaceMismatch {
            monitor_name: mon_dp1,
            special_workspace_id: ws_id,
            workspace_monitor: Some(mon_hdmi),
        };
        assert_eq!(
            inc5.to_string(),
            "monitor 'DP-1' special workspace 1 is assigned to monitor 'HDMI-1' instead of 'DP-1'"
        );
    }

    #[test]
    fn test_find_state_inconsistencies_consistent() {
        let mut workspaces = std::collections::BTreeMap::new();
        workspaces.insert(
            WorkspaceId::new(1),
            Workspace::new(
                WorkspaceId::new(1),
                WorkspaceName::new("1"),
                Some(MonitorName::new("DP-1")),
            ),
        );
        let mut monitors = std::collections::BTreeMap::new();
        monitors.insert(
            MonitorName::new("DP-1"),
            Monitor::new(MonitorName::new("DP-1"), WorkspaceId::new(1), None),
        );

        let state = HyprlandState::new(workspaces, monitors, Some(MonitorName::new("DP-1")));
        let incs = HyprlandAdapter::find_state_inconsistencies(&state);
        assert!(incs.is_empty());
    }

    #[test]
    fn test_find_state_inconsistencies_workspace_missing_monitor() {
        let mut workspaces = std::collections::BTreeMap::new();
        workspaces.insert(
            WorkspaceId::new(2),
            Workspace::new(WorkspaceId::new(2), WorkspaceName::new("2"), None),
        );
        let mut monitors = std::collections::BTreeMap::new();
        monitors.insert(
            MonitorName::new("DP-1"),
            Monitor::new(MonitorName::new("DP-1"), WorkspaceId::new(2), None),
        );

        let state = HyprlandState::new(workspaces, monitors, Some(MonitorName::new("DP-1")));
        let incs = HyprlandAdapter::find_state_inconsistencies(&state);
        assert_eq!(
            incs,
            vec![
                StateInconsistency::WorkspaceMissingMonitor {
                    workspace_id: WorkspaceId::new(2),
                    workspace_name: WorkspaceName::new("2"),
                },
                StateInconsistency::MonitorActiveWorkspaceMismatch {
                    monitor_name: MonitorName::new("DP-1"),
                    workspace_id: WorkspaceId::new(2),
                    workspace_monitor: None,
                }
            ]
        );
    }

    #[test]
    fn test_find_state_inconsistencies_active_workspace_not_found() {
        let workspaces = std::collections::BTreeMap::new();
        let mut monitors = std::collections::BTreeMap::new();
        monitors.insert(
            MonitorName::new("DP-1"),
            Monitor::new(MonitorName::new("DP-1"), WorkspaceId::new(99), None),
        );

        let state = HyprlandState::new(workspaces, monitors, Some(MonitorName::new("DP-1")));
        let incs = HyprlandAdapter::find_state_inconsistencies(&state);
        assert_eq!(
            incs,
            vec![StateInconsistency::MonitorActiveWorkspaceNotFound {
                monitor_name: MonitorName::new("DP-1"),
                workspace_id: WorkspaceId::new(99),
            }]
        );
    }

    #[test]
    fn test_find_state_inconsistencies_active_workspace_mismatch() {
        let mut workspaces = std::collections::BTreeMap::new();
        workspaces.insert(
            WorkspaceId::new(1),
            Workspace::new(
                WorkspaceId::new(1),
                WorkspaceName::new("1"),
                Some(MonitorName::new("HDMI-1")),
            ),
        );
        let mut monitors = std::collections::BTreeMap::new();
        monitors.insert(
            MonitorName::new("DP-1"),
            Monitor::new(MonitorName::new("DP-1"), WorkspaceId::new(1), None),
        );

        let state = HyprlandState::new(workspaces, monitors, Some(MonitorName::new("DP-1")));
        let incs = HyprlandAdapter::find_state_inconsistencies(&state);
        assert_eq!(
            incs,
            vec![StateInconsistency::MonitorActiveWorkspaceMismatch {
                monitor_name: MonitorName::new("DP-1"),
                workspace_id: WorkspaceId::new(1),
                workspace_monitor: Some(MonitorName::new("HDMI-1")),
            }]
        );
    }

    #[test]
    fn test_find_state_inconsistencies_special_workspace_checks() {
        // Special workspace not found
        let mut workspaces = std::collections::BTreeMap::new();
        workspaces.insert(
            WorkspaceId::new(1),
            Workspace::new(
                WorkspaceId::new(1),
                WorkspaceName::new("1"),
                Some(MonitorName::new("DP-1")),
            ),
        );
        let mut monitors = std::collections::BTreeMap::new();
        monitors.insert(
            MonitorName::new("DP-1"),
            Monitor::new(
                MonitorName::new("DP-1"),
                WorkspaceId::new(1),
                Some(WorkspaceId::new(-99)),
            ),
        );

        let state = HyprlandState::new(workspaces, monitors, Some(MonitorName::new("DP-1")));
        let incs = HyprlandAdapter::find_state_inconsistencies(&state);
        assert_eq!(
            incs,
            vec![StateInconsistency::MonitorSpecialWorkspaceNotFound {
                monitor_name: MonitorName::new("DP-1"),
                special_workspace_id: WorkspaceId::new(-99),
            }]
        );

        // Special workspace monitor mismatch
        let mut workspaces2 = std::collections::BTreeMap::new();
        workspaces2.insert(
            WorkspaceId::new(1),
            Workspace::new(
                WorkspaceId::new(1),
                WorkspaceName::new("1"),
                Some(MonitorName::new("DP-1")),
            ),
        );
        workspaces2.insert(
            WorkspaceId::new(-99),
            Workspace::new(
                WorkspaceId::new(-99),
                WorkspaceName::new("special"),
                Some(MonitorName::new("HDMI-1")),
            ),
        );
        let mut monitors2 = std::collections::BTreeMap::new();
        monitors2.insert(
            MonitorName::new("DP-1"),
            Monitor::new(
                MonitorName::new("DP-1"),
                WorkspaceId::new(1),
                Some(WorkspaceId::new(-99)),
            ),
        );

        let state2 = HyprlandState::new(workspaces2, monitors2, Some(MonitorName::new("DP-1")));
        let incs2 = HyprlandAdapter::find_state_inconsistencies(&state2);
        assert_eq!(
            incs2,
            vec![StateInconsistency::MonitorSpecialWorkspaceMismatch {
                monitor_name: MonitorName::new("DP-1"),
                special_workspace_id: WorkspaceId::new(-99),
                workspace_monitor: Some(MonitorName::new("HDMI-1")),
            }]
        );
    }
}
