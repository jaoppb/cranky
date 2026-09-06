use crate::features::workspaces::adapters::inconsistency::{
    find_state_inconsistencies, StateInconsistency,
};
use crate::features::workspaces::domain::{
    Monitor, MonitorName, Workspace, WorkspaceId, WorkspaceName,
};
use crate::shared::events::signals::HyprlandState;

#[test]
fn test_state_inconsistency_display() {
    let ws_id = WorkspaceId::new(1);
    let ws_name = WorkspaceName::new("code");
    let mon_dp1 = MonitorName::new("DP-1");
    let mon_hdmi = MonitorName::new("HDMI-1");

    let inc1 = StateInconsistency::WorkspaceMissingMonitor {
        workspace_id: ws_id.clone(),
        workspace_name: ws_name,
    };
    assert_eq!(
        inc1.to_string(),
        "workspace 1 ('code') has no monitor assigned"
    );

    let inc2 = StateInconsistency::MonitorActiveWorkspaceNotFound {
        monitor_name: mon_dp1.clone(),
        workspace_id: ws_id.clone(),
    };
    assert_eq!(
        inc2.to_string(),
        "monitor 'DP-1' active workspace 1 is not present in known workspaces"
    );

    let inc3 = StateInconsistency::MonitorActiveWorkspaceMismatch {
        monitor_name: mon_dp1.clone(),
        workspace_id: ws_id.clone(),
        workspace_monitor: Some(mon_hdmi.clone()),
    };
    assert_eq!(
        inc3.to_string(),
        "monitor 'DP-1' active workspace 1 is assigned to monitor 'HDMI-1' instead of 'DP-1'"
    );

    let inc3_none = StateInconsistency::MonitorActiveWorkspaceMismatch {
        monitor_name: mon_dp1.clone(),
        workspace_id: ws_id.clone(),
        workspace_monitor: None,
    };
    assert_eq!(
        inc3_none.to_string(),
        "monitor 'DP-1' active workspace 1 is assigned to monitor 'None' instead of 'DP-1'"
    );

    let inc4 = StateInconsistency::MonitorSpecialWorkspaceNotFound {
        monitor_name: mon_dp1.clone(),
        special_workspace_id: ws_id.clone(),
    };
    assert_eq!(
        inc4.to_string(),
        "monitor 'DP-1' special workspace 1 is not present in known workspaces"
    );

    let inc5 = StateInconsistency::MonitorSpecialWorkspaceMismatch {
        monitor_name: mon_dp1,
        special_workspace_id: ws_id,
        workspace_monitor: Some(mon_hdmi),
    };
    assert_eq!(
        inc5.to_string(),
        "monitor 'DP-1' special workspace 1 is assigned to monitor 'HDMI-1' instead of 'DP-1'"
    );
}

#[test]
fn test_find_state_inconsistencies_consistent() {
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
        Monitor::new(MonitorName::new("DP-1"), WorkspaceId::new(1), None),
    );

    let state = HyprlandState::new(workspaces, monitors, Some(MonitorName::new("DP-1")));
    let incs = find_state_inconsistencies(&state);
    assert!(incs.is_empty());
}
