use super::dto::{HyprMonitorDto, HyprWorkspaceDto};
use super::event_parser::parse_event;
use super::hyprland_provider::{HyprlandProvider, RealHyprlandProvider};
use super::inconsistency::{StateInconsistency, find_state_inconsistencies};
use super::signal_loop::run_event_loop;
use crate::features::workspaces::ports::{
    WindowManagerError, WindowManagerPort, WindowManagerState,
};
use crate::shared::events::signals::{HyprlandState, SignalHub};
use std::sync::Arc;

#[derive(Clone)]
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

    #[must_use]
    pub fn parse_event(raw_line: &str) -> Option<crate::shared::events::core::WindowManagerEvent> {
        parse_event(raw_line)
    }

    #[must_use]
    pub fn find_state_inconsistencies(state: &HyprlandState) -> Vec<StateInconsistency> {
        find_state_inconsistencies(state)
    }

    /// Runs a background loop that listens to Hyprland event socket and pushes updates to the `SignalHub`.
    pub async fn run(self, hub: Arc<SignalHub>) {
        run_event_loop(self.clone(), self.provider.clone(), hub).await;
    }
}

impl WindowManagerPort for HyprlandAdapter {
    fn get_state(&self) -> Result<WindowManagerState, WindowManagerError> {
        let _span = tracing::info_span!("get_state").entered();
        self.query_state()
            .inspect_err(|error| tracing::error!(%error))
    }
}

impl HyprlandAdapter {
    fn query_state(&self) -> Result<WindowManagerState, WindowManagerError> {
        let ws_json =
            self.provider
                .query_workspaces()
                .map_err(|error| WindowManagerError::IpcError {
                    reason: format!("Failed to get workspaces: {error}"),
                })?;
        let mon_json =
            self.provider
                .query_monitors()
                .map_err(|error| WindowManagerError::IpcError {
                    reason: format!("Failed to get monitors: {error}"),
                })?;

        let workspaces: std::collections::BTreeMap<_, _> =
            serde_json::from_str::<Vec<HyprWorkspaceDto>>(&ws_json)
                .map_err(|error| WindowManagerError::IpcError {
                    reason: error.to_string(),
                })?
                .into_iter()
                .map(|dto| {
                    let workspace = dto.into_domain();
                    (workspace.id().clone(), workspace)
                })
                .collect();

        let mut focused_monitor = None;
        let monitors: std::collections::BTreeMap<_, _> =
            serde_json::from_str::<Vec<HyprMonitorDto>>(&mon_json)
                .map_err(|error| WindowManagerError::IpcError {
                    reason: error.to_string(),
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
