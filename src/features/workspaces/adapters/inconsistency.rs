use crate::features::workspaces::domain::{MonitorName, WorkspaceId, WorkspaceName};
use crate::shared::events::signals::HyprlandState;
use serde::Serialize;

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
