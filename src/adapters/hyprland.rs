use crate::core::hyprland::{HyprlandProvider, RealHyprlandProvider};
use crate::domain::signals::{HyprlandState, SignalHub};
use crate::domain::workspace::{Monitor, Workspace};
use crate::ports::WindowManagerError;
use crate::ports::WindowManagerPort;
use serde::Deserialize;
use std::sync::Arc;

#[derive(Deserialize)]
struct HyprWorkspaceDto {
    id: i32,
    name: String,
    monitor: String,
}

impl HyprWorkspaceDto {
    fn into_domain(self) -> Workspace {
        Workspace::new(
            crate::domain::workspace::WorkspaceId::new(self.id),
            crate::domain::workspace::WorkspaceName::new(self.name),
            Some(crate::domain::workspace::MonitorName::new(self.monitor)),
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
    fn into_domain(self) -> (Monitor, bool) {
        let special_ws_id = if self.special_workspace.id != 0 {
            Some(crate::domain::workspace::WorkspaceId::new(self.special_workspace.id))
        } else {
            None
        };
        let monitor = Monitor::new(
            crate::domain::workspace::MonitorName::new(self.name),
            crate::domain::workspace::WorkspaceId::new(self.active_workspace.id),
            special_ws_id,
        );
        (monitor, self.focused)
    }
}

pub struct HyprlandAdapter {
    provider: Box<dyn HyprlandProvider>,
}

impl HyprlandAdapter {
    pub fn new() -> Self {
        Self {
            provider: Box::new(RealHyprlandProvider),
        }
    }

    fn parse_event(raw_line: &str) -> Option<crate::domain::events::WindowManagerEvent> {
        use crate::domain::events::{WindowManagerEvent, WindowAddress, WindowTitle};
        use crate::domain::workspace::{MonitorName, WorkspaceId, WorkspaceName};

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
                let parts: Vec<&str> = payload.splitn(3, ',').collect();
                if parts.len() != 3 { return None; }
                Some(WindowManagerEvent::WorkspaceMoved {
                    id: WorkspaceId::new(parts[0].parse().ok()?),
                    name: WorkspaceName::new(parts[1]),
                    monitor_name: MonitorName::new(parts[2]),
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
                let parts: Vec<&str> = payload.splitn(3, ',').collect();
                if parts.len() != 3 { return None; }
                let id = match parts[0].parse::<i32>() {
                    Ok(0) | Err(_) => None,
                    Ok(val) => Some(WorkspaceId::new(val)),
                };
                let ws_name = if id.is_none() || parts[1].is_empty() { None } else { Some(WorkspaceName::new(parts[1])) };
                Some(WindowManagerEvent::SpecialWorkspaceActivated {
                    id,
                    name: ws_name,
                    monitor_name: MonitorName::new(parts[2]),
                })
            }
            "activewindowv2" => {
                Some(WindowManagerEvent::ActiveWindowChanged {
                    address: WindowAddress::new(payload),
                })
            }
            "windowtitlev2" => {
                let (address, title) = payload.split_once(',')?;
                Some(WindowManagerEvent::WindowTitleChanged {
                    address: WindowAddress::new(address),
                    title: WindowTitle::new(title),
                })
            }
            _ => None
        }
    }

    /// Runs a background loop that listens to Hyprland event socket and pushes updates to the SignalHub.
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
                        tracing::error!("Failed to connect to Hyprland event socket: {}", e);
                        std::thread::sleep(std::time::Duration::from_millis(1000));
                        continue;
                    }
                };
                
                // set read timeout to simulate batching drain (2ms)
                let _ = stream.set_read_timeout(Some(std::time::Duration::from_millis(2)));
                
                use std::io::{BufRead, ErrorKind};
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
                        tracing::error!("Hyprland adapter error on initial fetch: {}", e);
                        HyprlandState::new(std::collections::BTreeMap::new(), std::collections::BTreeMap::new(), None)
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
                            let mut state_changed = false;
                            if let Some(event) = Self::parse_event(&line) {
                                current_state.apply_event(&event);
                                state_changed = true;
                            }

                            // Now, quickly drain any other immediately pending events (batching)
                            let _ = reader.get_mut().set_read_timeout(Some(std::time::Duration::from_millis(2)));
                            loop {
                                line.clear();
                                match reader.read_line(&mut line) {
                                    Ok(0) => break, // EOF
                                    Ok(_) => {
                                        if let Some(event) = Self::parse_event(&line) {
                                            current_state.apply_event(&event);
                                            state_changed = true;
                                        }
                                    }
                                    Err(e) if e.kind() == ErrorKind::WouldBlock || e.kind() == ErrorKind::TimedOut => {
                                        break; // End of burst
                                    }
                                    Err(_) => break, // Other error
                                }
                            }

                            // Validate state consistency before broadcast
                            if state_changed {
                                let mut inconsistent = false;
                                for ws in current_state.workspaces().values() {
                                    if ws.monitor().is_none() {
                                        inconsistent = true;
                                        break;
                                    }
                                }
                                if !inconsistent {
                                    for mon in current_state.monitors().values() {
                                        if let Some(ws) = current_state.workspaces().get(mon.active_workspace_id()) {
                                            if ws.monitor() != Some(mon.name()) {
                                                inconsistent = true;
                                                break;
                                            }
                                        } else {
                                            // Missing workspace
                                            inconsistent = true;
                                            break;
                                        }
                                        if let Some(sp_id) = mon.special_workspace_id() {
                                            if let Some(ws) = current_state.workspaces().get(sp_id) {
                                                if ws.monitor() != Some(mon.name()) {
                                                    inconsistent = true;
                                                    break;
                                                }
                                            } else {
                                                inconsistent = true;
                                                break;
                                            }
                                        }
                                    }
                                }
                                
                                if inconsistent {
                                    tracing::warn!("Hyprland state inconsistent after event batch, forcing full resync");
                                    if let Ok((workspaces, monitors, focused)) = self.get_state() {
                                        current_state = HyprlandState::new(workspaces, monitors, focused);
                                    }
                                }
                            }

                            // Broadcast reduced state
                            if state_changed && *hypr_tx.borrow() != current_state {
                                let _ = hypr_tx.send(current_state.clone());
                            }
                        }
                        Err(e) => {
                            tracing::error!("Hyprland event socket read error: {}", e);
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
    fn get_state(&self) -> Result<crate::ports::WindowManagerState, WindowManagerError> {
        let ws_json =
            self.provider
                .query_workspaces()
                .map_err(|e| WindowManagerError::IpcError {
                    reason: format!("Failed to get workspaces: {}", e),
                })?;
        let mon_json =
            self.provider
                .query_monitors()
                .map_err(|e| WindowManagerError::IpcError {
                    reason: format!("Failed to get monitors: {}", e),
                })?;

        let workspaces: std::collections::BTreeMap<_, _> = serde_json::from_str::<Vec<HyprWorkspaceDto>>(&ws_json)
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
        let monitors: std::collections::BTreeMap<_, _> = serde_json::from_str::<Vec<HyprMonitorDto>>(&mon_json)
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
    use crate::core::hyprland::MockHyprlandProvider;

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

        let adapter = HyprlandAdapter {
            provider: Box::new(mock_provider),
        };

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

        let adapter = HyprlandAdapter {
            provider: Box::new(mock_provider),
        };

        let res = adapter.get_state().unwrap();
        assert_eq!(res.0.len(), 1); // workspaces
        assert_eq!(res.1.len(), 1); // monitors
        assert_eq!(res.2, Some(crate::domain::workspace::MonitorName::new("DP-1"))); // focused
    }

    #[test]
    fn test_parse_event() {
        use crate::domain::events::{WindowManagerEvent, WindowAddress, WindowTitle};
        use crate::domain::workspace::{MonitorName, WorkspaceId, WorkspaceName};

        // workspacev2
        let e = HyprlandAdapter::parse_event("workspacev2>>1,test_ws").unwrap();
        assert_eq!(e, WindowManagerEvent::WorkspaceActivated {
            id: WorkspaceId::new(1),
            name: WorkspaceName::new("test_ws"),
        });

        // focusedmonv2
        let e = HyprlandAdapter::parse_event("focusedmonv2>>DP-1,2").unwrap();
        assert_eq!(e, WindowManagerEvent::MonitorFocused {
            monitor_name: MonitorName::new("DP-1"),
            workspace_id: WorkspaceId::new(2),
        });

        // createworkspacev2
        let e = HyprlandAdapter::parse_event("createworkspacev2>>3,new_ws").unwrap();
        assert_eq!(e, WindowManagerEvent::WorkspaceCreated {
            id: WorkspaceId::new(3),
            name: WorkspaceName::new("new_ws"),
        });

        // destroyworkspacev2
        let e = HyprlandAdapter::parse_event("destroyworkspacev2>>4,old_ws").unwrap();
        assert_eq!(e, WindowManagerEvent::WorkspaceDestroyed {
            id: WorkspaceId::new(4),
            name: WorkspaceName::new("old_ws"),
        });

        // moveworkspacev2
        let e = HyprlandAdapter::parse_event("moveworkspacev2>>5,moved_ws,HDMI-1").unwrap();
        assert_eq!(e, WindowManagerEvent::WorkspaceMoved {
            id: WorkspaceId::new(5),
            name: WorkspaceName::new("moved_ws"),
            monitor_name: MonitorName::new("HDMI-1"),
        });

        // renameworkspace
        let e = HyprlandAdapter::parse_event("renameworkspace>>6,renamed_ws").unwrap();
        assert_eq!(e, WindowManagerEvent::WorkspaceRenamed {
            id: WorkspaceId::new(6),
            new_name: WorkspaceName::new("renamed_ws"),
        });

        // activespecialv2
        let e = HyprlandAdapter::parse_event("activespecialv2>>7,special,DP-2").unwrap();
        assert_eq!(e, WindowManagerEvent::SpecialWorkspaceActivated {
            id: Some(WorkspaceId::new(7)),
            name: Some(WorkspaceName::new("special")),
            monitor_name: MonitorName::new("DP-2"),
        });

        // activespecialv2 closed (0 ID)
        let e = HyprlandAdapter::parse_event("activespecialv2>>0,,DP-2").unwrap();
        assert_eq!(e, WindowManagerEvent::SpecialWorkspaceActivated {
            id: None,
            name: None,
            monitor_name: MonitorName::new("DP-2"),
        });

        // activespecialv2 negative ID
        let e = HyprlandAdapter::parse_event("activespecialv2>>-98,special:magic,DP-2").unwrap();
        assert_eq!(e, WindowManagerEvent::SpecialWorkspaceActivated {
            id: Some(WorkspaceId::new(-98)),
            name: Some(WorkspaceName::new("special:magic")),
            monitor_name: MonitorName::new("DP-2"),
        });

        // activewindowv2
        let e = HyprlandAdapter::parse_event("activewindowv2>>0x123").unwrap();
        assert_eq!(e, WindowManagerEvent::ActiveWindowChanged {
            address: WindowAddress::new("0x123"),
        });

        // windowtitlev2
        let e = HyprlandAdapter::parse_event("windowtitlev2>>0x456,Title").unwrap();
        assert_eq!(e, WindowManagerEvent::WindowTitleChanged {
            address: WindowAddress::new("0x456"),
            title: WindowTitle::new("Title"),
        });

        // invalid
        assert_eq!(HyprlandAdapter::parse_event("invalid>>data"), None);
        assert_eq!(HyprlandAdapter::parse_event("missing"), None);
    }

    #[tokio::test]
    async fn test_hyprland_adapter_run() {
        use std::os::unix::net::UnixStream;
        use std::io::Write;
        
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

        let adapter = HyprlandAdapter {
            provider: Box::new(mock_provider),
        };

        let config = crate::domain::config::Config::default();
        let hub = Arc::new(SignalHub::new(config));
        
        let run_handle = tokio::task::spawn(adapter.run(hub.clone()));

        sender.write_all(b"workspacev2>>1,test_ws\n").unwrap();
        
        tokio::time::sleep(std::time::Duration::from_millis(150)).await;
        
        drop(sender);
        
        let _ = run_handle.await;
    }
}
