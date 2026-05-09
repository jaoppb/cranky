use crate::ports::WindowManagerPort;
use crate::domain::errors::PortError;
use crate::domain::signals::{SignalHub, HyprlandState};
use crate::core::hyprland::{Workspace, Monitor, RealHyprlandProvider, HyprlandProvider};
use std::sync::Arc;
use tokio::sync::watch;
use tracing::{error, debug_span};
use std::time::Duration;

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
    fn get_state(&self) -> Result<(Vec<Workspace>, Vec<Monitor>), PortError> {
        let workspaces = self.provider.get_workspaces().map_err(|e| PortError::WindowManagerIpcError { 
            reason: format!("Failed to get workspaces: {}", e) 
        })?;
        let monitors = self.provider.get_monitors().map_err(|e| PortError::WindowManagerIpcError { 
            reason: format!("Failed to get monitors: {}", e) 
        })?;
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
        mock_provider.expect_get_workspaces()
            .times(1)
            .returning(|| Ok(Vec::new()));
        mock_provider.expect_get_monitors()
            .times(1)
            .returning(|| Ok(Vec::new()));

        let adapter = HyprlandAdapter {
            provider: Box::new(mock_provider),
        };

        let res = adapter.get_state().unwrap();
        assert_eq!(res.0.len(), 0);
        assert_eq!(res.1.len(), 0);
    }
}
