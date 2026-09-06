use crate::features::workspaces::adapters::inconsistency::{
    find_state_inconsistencies, StateInconsistency,
};
use crate::features::workspaces::domain::{
    Monitor, MonitorName, Workspace, WorkspaceId, WorkspaceName,
};
use crate::shared::events::signals::HyprlandState;

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
fn test_find_state_inconsistencies_active_workspace_not_found() {
    let workspaces = std::collections::BTreeMap::new();
    let mut monitors = std::collections::BTreeMap::new();
    monitors.insert(
        MonitorName::new("DP-1"),
        Monitor::new(MonitorName::new("DP-1"), WorkspaceId::new(99), None),
    );

    let state = HyprlandState::new(workspaces, monitors, Some(MonitorName::new("DP-1")));
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
    let incs = find_state_inconsistencies(&state);
    assert_eq!(
        incs,
        vec![StateInconsistency::MonitorSpecialWorkspaceNotFound {
            monitor_name: MonitorName::new("DP-1"),
            special_workspace_id: WorkspaceId::new(-99),
        }]
    );

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
