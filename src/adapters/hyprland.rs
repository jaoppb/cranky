use crate::ports::WindowManagerPort;
use crate::ports::WindowManagerError;
use crate::domain::signals::{SignalHub, HyprlandState};
use crate::domain::workspace::{Workspace, Monitor};
use crate::core::hyprland::{RealHyprlandProvider, HyprlandProvider};
use std::sync::Arc;
use serde::Deserialize;
use tracing::{error, debug_span};
use std::time::Duration;

#[derive(Deserialize)]
struct HyprWorkspaceDto {
    id: i32,
    monitor: String,
}

impl HyprWorkspaceDto {
    fn to_domain(self) -> Workspace {
        Workspace::new(self.id, self.monitor)
    }
}

#[derive(Deserialize)]
struct HyprMonitorDto {
    name: String,
    #[serde(rename = "activeWorkspace")]
    active_workspace: HyprActiveWorkspaceDto,
    focused: bool,
}

#[derive(Deserialize)]
struct HyprActiveWorkspaceDto {
    id: i32,
}

impl HyprMonitorDto {
    fn to_domain(self) -> Monitor {
        Monitor::new(self.name, self.active_workspace.id, self.focused)
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

    /// Runs a background loop that polls Hyprland state and pushes updates to the SignalHub.
    pub async fn run(&self, hub: Arc<SignalHub>) {
        let hypr_tx = hub.hyprland_tx();
        let mut interval = tokio::time::interval(Duration::from_millis(100));

        loop {
            interval.tick().await;
            let poll_span = debug_span!("hyprland_poll");
            let _enter = poll_span.enter();
            match self.get_state() {
                Ok((workspaces, monitors)) => {
                    let new_state = HyprlandState::new(workspaces, monitors);
                    if *hypr_tx.borrow() != new_state {
                        let _ = hypr_tx.send(new_state);
                    }
                }
                Err(e) => {
                    error!("Hyprland adapter error: {}", e);
                }
            }
        }
    }
}

impl WindowManagerPort for HyprlandAdapter {
    fn get_state(&self) -> Result<(Vec<Workspace>, Vec<Monitor>), WindowManagerError> {
        let ws_json = self.provider.query_workspaces().map_err(|e| WindowManagerError::IpcError { 
            reason: format!("Failed to get workspaces: {}", e) 
        })?;
        let mon_json = self.provider.query_monitors().map_err(|e| WindowManagerError::IpcError { 
            reason: format!("Failed to get monitors: {}", e) 
        })?;
        
        let workspaces: Vec<Workspace> = serde_json::from_str::<Vec<HyprWorkspaceDto>>(&ws_json)
            .map_err(|e| WindowManagerError::IpcError { reason: e.to_string() })?
            .into_iter()
            .map(HyprWorkspaceDto::to_domain)
            .collect();

        let monitors: Vec<Monitor> = serde_json::from_str::<Vec<HyprMonitorDto>>(&mon_json)
            .map_err(|e| WindowManagerError::IpcError { reason: e.to_string() })?
            .into_iter()
            .map(HyprMonitorDto::to_domain)
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
        mock_provider.expect_query_workspaces()
            .times(1)
            .returning(|| Ok("[]".to_string()));
        mock_provider.expect_query_monitors()
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
