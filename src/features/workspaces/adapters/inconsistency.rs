//! Checks a Hyprland state snapshot for inconsistencies.
//!
//! It flags references that do not resolve (such as a monitor whose active workspace is missing),
//! a workspace with no monitor, and monitor and workspace records that disagree.
use crate::features::workspaces::domain::{
    Monitor, MonitorName, Workspace, WorkspaceId, WorkspaceName,
};
use crate::shared::events::signals::HyprlandState;
use serde::Serialize;

/// Represents a specific consistency failure in the internal Hyprland state.
///
/// Its `Display` text, derived by `thiserror`, is what an inconsistency reads as in a message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, thiserror::Error)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum StateInconsistency {
    /// A workspace exists in state but has no assigned monitor.
    #[error("workspace {workspace_id} ('{workspace_name}') has no monitor assigned")]
    WorkspaceMissingMonitor {
        workspace_id: WorkspaceId,
        workspace_name: WorkspaceName,
    },
    /// A workspace's assigned monitor does not exist in the state.
    #[error(
        "workspace {workspace_id} ('{workspace_name}') is assigned to monitor '{monitor_name}' which does not exist"
    )]
    WorkspaceMonitorNotFound {
        workspace_id: WorkspaceId,
        workspace_name: WorkspaceName,
        monitor_name: MonitorName,
    },
    /// A monitor references an active workspace that does not exist in the state.
    #[error(
        "monitor '{monitor_name}' active workspace {workspace_id} is not present in known workspaces"
    )]
    MonitorActiveWorkspaceNotFound {
        monitor_name: MonitorName,
        workspace_id: WorkspaceId,
    },
    /// A monitor references an active workspace whose assigned monitor differs from the monitor itself.
    #[error(
        "monitor '{monitor_name}' active workspace {workspace_id} is assigned to monitor '{actual}' instead of '{monitor_name}'",
        actual = monitor_or_none(.workspace_monitor.as_ref())
    )]
    MonitorActiveWorkspaceMismatch {
        monitor_name: MonitorName,
        workspace_id: WorkspaceId,
        workspace_monitor: Option<MonitorName>,
    },
    /// A monitor references a special workspace that does not exist in the state.
    #[error(
        "monitor '{monitor_name}' special workspace {special_workspace_id} is not present in known workspaces"
    )]
    MonitorSpecialWorkspaceNotFound {
        monitor_name: MonitorName,
        special_workspace_id: WorkspaceId,
    },
    /// A monitor references a special workspace whose assigned monitor differs from the monitor itself.
    #[error(
        "monitor '{monitor_name}' special workspace {special_workspace_id} is assigned to monitor '{actual}' instead of '{monitor_name}'",
        actual = monitor_or_none(.workspace_monitor.as_ref())
    )]
    MonitorSpecialWorkspaceMismatch {
        monitor_name: MonitorName,
        special_workspace_id: WorkspaceId,
        workspace_monitor: Option<MonitorName>,
    },
}

/// The name a mismatch message gives a workspace's monitor: the monitor's name, or `None`.
fn monitor_or_none(workspace_monitor: Option<&MonitorName>) -> &str {
    workspace_monitor.map_or("None", MonitorName::as_str)
}

/// Checks for inconsistencies in the current Hyprland state.
///
/// Returns a list of structured [`StateInconsistency`] items found.
#[must_use]
pub(super) fn find_state_inconsistencies(state: &HyprlandState) -> Vec<StateInconsistency> {
    let mut inconsistencies = Vec::new();
    for (ws_id, workspace) in state.workspaces() {
        inconsistencies.extend(check_workspace_monitor(state, ws_id, workspace));
    }
    for (mon_name, mon) in state.monitors() {
        inconsistencies.extend(check_active_workspace(state, mon_name, mon));
        inconsistencies.extend(check_special_workspace(state, mon_name, mon));
    }
    inconsistencies
}

/// Flags a workspace with no monitor, or whose monitor is not in the state.
fn check_workspace_monitor(
    state: &HyprlandState,
    ws_id: &WorkspaceId,
    workspace: &Workspace,
) -> Option<StateInconsistency> {
    match workspace.monitor() {
        None => Some(StateInconsistency::WorkspaceMissingMonitor {
            workspace_id: ws_id.clone(),
            workspace_name: workspace.name().clone(),
        }),
        Some(monitor_name) if !state.monitors().contains_key(monitor_name) => {
            Some(StateInconsistency::WorkspaceMonitorNotFound {
                workspace_id: ws_id.clone(),
                workspace_name: workspace.name().clone(),
                monitor_name: monitor_name.clone(),
            })
        }
        Some(_) => None,
    }
}

/// Flags a monitor whose active workspace is missing, or sits on another monitor.
fn check_active_workspace(
    state: &HyprlandState,
    mon_name: &MonitorName,
    mon: &Monitor,
) -> Option<StateInconsistency> {
    let Some(workspace) = state.workspaces().get(mon.active_workspace_id()) else {
        return Some(StateInconsistency::MonitorActiveWorkspaceNotFound {
            monitor_name: mon_name.clone(),
            workspace_id: mon.active_workspace_id().clone(),
        });
    };
    (workspace.monitor() != Some(mon.name())).then(|| {
        StateInconsistency::MonitorActiveWorkspaceMismatch {
            monitor_name: mon_name.clone(),
            workspace_id: mon.active_workspace_id().clone(),
            workspace_monitor: workspace.monitor().cloned(),
        }
    })
}

/// Flags a monitor whose special workspace, when it has one, is missing or sits on another
/// monitor.
fn check_special_workspace(
    state: &HyprlandState,
    mon_name: &MonitorName,
    mon: &Monitor,
) -> Option<StateInconsistency> {
    let sp_id = mon.special_workspace_id()?;
    let Some(special_workspace) = state.workspaces().get(sp_id) else {
        return Some(StateInconsistency::MonitorSpecialWorkspaceNotFound {
            monitor_name: mon_name.clone(),
            special_workspace_id: sp_id.clone(),
        });
    };
    (special_workspace.monitor() != Some(mon.name())).then(|| {
        StateInconsistency::MonitorSpecialWorkspaceMismatch {
            monitor_name: mon_name.clone(),
            special_workspace_id: sp_id.clone(),
            workspace_monitor: special_workspace.monitor().cloned(),
        }
    })
}
