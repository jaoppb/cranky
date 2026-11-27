use super::fixtures::{monitor, state, workspace};
use crate::features::workspaces::adapters::inconsistency::{
    StateInconsistency, find_state_inconsistencies,
};
use crate::features::workspaces::domain::{MonitorName, WorkspaceId, WorkspaceName};

/// Checks that `inconsistency` reads as `expected`.
fn assert_displays(inconsistency: &StateInconsistency, expected: &str) {
    assert_eq!(inconsistency.to_string(), expected);
}

#[test]
fn test_state_inconsistency_display() {
    let ws_id = WorkspaceId::new(1);
    let mon_dp1 = MonitorName::new("DP-1");

    assert_displays(
        &StateInconsistency::WorkspaceMissingMonitor {
            workspace_id: ws_id.clone(),
            workspace_name: WorkspaceName::new("code"),
        },
        "workspace 1 ('code') has no monitor assigned",
    );
    assert_displays(
        &StateInconsistency::WorkspaceMonitorNotFound {
            workspace_id: ws_id.clone(),
            workspace_name: WorkspaceName::new("code"),
            monitor_name: mon_dp1.clone(),
        },
        "workspace 1 ('code') is assigned to monitor 'DP-1' which does not exist",
    );
    assert_displays(
        &StateInconsistency::MonitorActiveWorkspaceNotFound {
            monitor_name: mon_dp1,
            workspace_id: ws_id,
        },
        "monitor 'DP-1' active workspace 1 is not present in known workspaces",
    );
}

#[test]
fn test_state_inconsistency_display_active_workspace_mismatch() {
    assert_displays(
        &StateInconsistency::MonitorActiveWorkspaceMismatch {
            monitor_name: MonitorName::new("DP-1"),
            workspace_id: WorkspaceId::new(1),
            workspace_monitor: Some(MonitorName::new("HDMI-1")),
        },
        "monitor 'DP-1' active workspace 1 is assigned to monitor 'HDMI-1' instead of 'DP-1'",
    );
    assert_displays(
        &StateInconsistency::MonitorActiveWorkspaceMismatch {
            monitor_name: MonitorName::new("DP-1"),
            workspace_id: WorkspaceId::new(1),
            workspace_monitor: None,
        },
        "monitor 'DP-1' active workspace 1 is assigned to monitor 'None' instead of 'DP-1'",
    );
}

#[test]
fn test_state_inconsistency_display_special_workspace() {
    assert_displays(
        &StateInconsistency::MonitorSpecialWorkspaceNotFound {
            monitor_name: MonitorName::new("DP-1"),
            special_workspace_id: WorkspaceId::new(1),
        },
        "monitor 'DP-1' special workspace 1 is not present in known workspaces",
    );
    assert_displays(
        &StateInconsistency::MonitorSpecialWorkspaceMismatch {
            monitor_name: MonitorName::new("DP-1"),
            special_workspace_id: WorkspaceId::new(1),
            workspace_monitor: Some(MonitorName::new("HDMI-1")),
        },
        "monitor 'DP-1' special workspace 1 is assigned to monitor 'HDMI-1' instead of 'DP-1'",
    );
}

#[test]
fn test_find_state_inconsistencies_consistent() {
    let state = state(
        vec![workspace(1, "1", Some("DP-1"))],
        vec![monitor("DP-1", 1, None)],
        Some("DP-1"),
    );
    let incs = find_state_inconsistencies(&state);
    assert_eq!(incs, []);
}

#[test]
fn test_state_inconsistency_display_special_mismatch_without_monitor() {
    assert_displays(
        &StateInconsistency::MonitorSpecialWorkspaceMismatch {
            monitor_name: MonitorName::new("DP-1"),
            special_workspace_id: WorkspaceId::new(-98),
            workspace_monitor: None,
        },
        "monitor 'DP-1' special workspace -98 is assigned to monitor 'None' instead of 'DP-1'",
    );
}

/// A special workspace that exists on its own monitor is consistent.
#[test]
fn test_find_state_inconsistencies_special_workspace_on_its_monitor() {
    let state = state(
        vec![
            workspace(1, "1", Some("DP-1")),
            workspace(-99, "special", Some("DP-1")),
        ],
        vec![monitor("DP-1", 1, Some(-99))],
        Some("DP-1"),
    );
    assert_eq!(find_state_inconsistencies(&state), []);
}

/// Workspace problems come first, then each monitor's active workspace
/// before its special workspace.
#[test]
fn test_find_state_inconsistencies_reports_in_order() {
    let state = state(
        vec![workspace(5, "5", None)],
        vec![monitor("DP-1", 99, Some(-99))],
        None,
    );
    assert_eq!(
        find_state_inconsistencies(&state),
        vec![
            StateInconsistency::WorkspaceMissingMonitor {
                workspace_id: WorkspaceId::new(5),
                workspace_name: WorkspaceName::new("5"),
            },
            StateInconsistency::MonitorActiveWorkspaceNotFound {
                monitor_name: MonitorName::new("DP-1"),
                workspace_id: WorkspaceId::new(99),
            },
            StateInconsistency::MonitorSpecialWorkspaceNotFound {
                monitor_name: MonitorName::new("DP-1"),
                special_workspace_id: WorkspaceId::new(-99),
            },
        ]
    );
}
