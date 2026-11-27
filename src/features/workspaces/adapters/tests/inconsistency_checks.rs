use super::fixtures::{monitor, state, workspace};
use crate::features::workspaces::adapters::inconsistency::{
    StateInconsistency, find_state_inconsistencies,
};
use crate::features::workspaces::domain::{MonitorName, WorkspaceId, WorkspaceName};

#[test]
fn test_find_state_inconsistencies_workspace_missing_monitor() {
    let state = state(
        vec![workspace(2, "2", None)],
        vec![monitor("DP-1", 2, None)],
        Some("DP-1"),
    );
    let incs = find_state_inconsistencies(&state);
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
fn test_find_state_inconsistencies_workspace_monitor_not_found() {
    // HDMI-A-1 was removed, but the workspace still points at it.
    let state = state(vec![workspace(4, "4", Some("HDMI-A-1"))], vec![], None);
    let incs = find_state_inconsistencies(&state);
    assert_eq!(
        incs,
        vec![StateInconsistency::WorkspaceMonitorNotFound {
            workspace_id: WorkspaceId::new(4),
            workspace_name: WorkspaceName::new("4"),
            monitor_name: MonitorName::new("HDMI-A-1"),
        }]
    );
}

#[test]
fn test_find_state_inconsistencies_active_workspace_not_found() {
    let state = state(vec![], vec![monitor("DP-1", 99, None)], Some("DP-1"));
    let incs = find_state_inconsistencies(&state);
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
    // HDMI-1 must exist too, otherwise WorkspaceMonitorNotFound also fires -
    // this test isolates the active-workspace mismatch check specifically.
    let state = state(
        vec![workspace(1, "1", Some("HDMI-1"))],
        vec![monitor("DP-1", 1, None), monitor("HDMI-1", 1, None)],
        Some("DP-1"),
    );
    let incs = find_state_inconsistencies(&state);
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
    let state = state(
        vec![workspace(1, "1", Some("DP-1"))],
        vec![monitor("DP-1", 1, Some(-99))],
        Some("DP-1"),
    );
    let incs = find_state_inconsistencies(&state);
    assert_eq!(
        incs,
        vec![StateInconsistency::MonitorSpecialWorkspaceNotFound {
            monitor_name: MonitorName::new("DP-1"),
            special_workspace_id: WorkspaceId::new(-99),
        }]
    );
}

#[test]
fn test_find_state_inconsistencies_special_workspace_mismatch() {
    // HDMI-1 must exist too, otherwise WorkspaceMonitorNotFound also fires -
    // this test isolates the special-workspace mismatch check specifically.
    let state2 = state(
        vec![
            workspace(1, "1", Some("DP-1")),
            workspace(-99, "special", Some("HDMI-1")),
        ],
        vec![monitor("DP-1", 1, Some(-99)), monitor("HDMI-1", -99, None)],
        Some("DP-1"),
    );
    let incs2 = find_state_inconsistencies(&state2);
    assert_eq!(
        incs2,
        vec![StateInconsistency::MonitorSpecialWorkspaceMismatch {
            monitor_name: MonitorName::new("DP-1"),
            special_workspace_id: WorkspaceId::new(-99),
            workspace_monitor: Some(MonitorName::new("HDMI-1")),
        }]
    );
}
