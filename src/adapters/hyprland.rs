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
            crate::domain::workspace::MonitorName::new(self.monitor),
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
    fn into_domain(self) -> Monitor {
        let special_ws_id = if self.special_workspace.id != 0 {
            Some(crate::domain::workspace::WorkspaceId::new(self.special_workspace.id))
        } else {
            None
        };
        Monitor::new(
            crate::domain::workspace::MonitorName::new(self.name),
            crate::domain::workspace::WorkspaceId::new(self.active_workspace.id),
            special_ws_id,
            self.focused,
        )
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

    /// Runs a background loop that listens to Hyprland event socket and pushes updates to the SignalHub.
    pub async fn run(self, hub: Arc<SignalHub>) {
        tokio::task::spawn_blocking(move || {
            let hypr_tx = hub.hyprland_tx();

            loop {
                let stream = match self.provider.listen_events() {
                    Ok(s) => s,
                    Err(e) => {
                        tracing::error!("Failed to connect to Hyprland event socket: {}", e);
                        std::thread::sleep(std::time::Duration::from_secs(1));
                        continue;
                    }
                };

                use std::io::BufRead;
                let mut reader = std::io::BufReader::new(stream);

                // Initial fetch before blocking on events
                let poll_span = tracing::debug_span!("hyprland_poll");
                let _enter = poll_span.enter();
                match self.get_state() {
                    Ok((workspaces, monitors)) => {
                        let new_state = HyprlandState::new(workspaces, monitors);
                        if *hypr_tx.borrow() != new_state {
                            let _ = hypr_tx.send(new_state);
                        }
                    }
                    Err(e) => {
                        tracing::error!("Hyprland adapter error on initial fetch: {}", e);
                    }
                }
                drop(_enter);

                let mut line = String::new();
                loop {
                    line.clear();
                    match reader.read_line(&mut line) {
                        Ok(0) => {
                            tracing::info!("Hyprland event socket closed, reconnecting...");
                            break;
                        }
                        Ok(_) => {
                            let poll_span = tracing::debug_span!("hyprland_poll");
                            let _enter = poll_span.enter();
                            match self.get_state() {
                                Ok((workspaces, monitors)) => {
                                    let new_state = HyprlandState::new(workspaces, monitors);
                                    if *hypr_tx.borrow() != new_state {
                                        let _ = hypr_tx.send(new_state);
                                    }
                                }
                                Err(e) => {
                                    tracing::error!(
                                        "Hyprland adapter error during event update: {}",
                                        e
                                    );
                                }
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
    fn get_state(&self) -> Result<(Vec<Workspace>, Vec<Monitor>), WindowManagerError> {
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

        let workspaces: Vec<Workspace> = serde_json::from_str::<Vec<HyprWorkspaceDto>>(&ws_json)
            .map_err(|e| WindowManagerError::IpcError {
                reason: e.to_string(),
            })?
            .into_iter()
            .map(HyprWorkspaceDto::into_domain)
            .collect();

        let monitors: Vec<Monitor> = serde_json::from_str::<Vec<HyprMonitorDto>>(&mon_json)
            .map_err(|e| WindowManagerError::IpcError {
                reason: e.to_string(),
            })?
            .into_iter()
            .map(HyprMonitorDto::into_domain)
            .collect();

        Ok((workspaces, monitors))
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
    }
}
