use super::identifiers::{MonitorName, WorkspaceId, WorkspaceName};
use super::monitor::Monitor;
use super::workspace::Workspace;

#[test]
fn test_workspace_id() {
    let id = WorkspaceId::new(42);
    assert_eq!(id, WorkspaceId::new(42));
}

#[test]
fn test_monitor_name() {
    let name = MonitorName::new("DP-1");
    assert_eq!(name.as_str(), "DP-1");
}

#[test]
fn test_workspace_name() {
    let name = WorkspaceName::new("1");
    assert_eq!(name, WorkspaceName::new("1"));
}

#[test]
fn test_workspace_operations() {
    let mut ws = Workspace::new(WorkspaceId::new(1), WorkspaceName::new("1"), None);
    assert_eq!(*ws.id(), WorkspaceId::new(1));
    assert_eq!(ws.name(), &WorkspaceName::new("1"));
    assert_eq!(ws.monitor(), None);

    ws.set_monitor(MonitorName::new("eDP-1"));
    assert_eq!(ws.monitor(), Some(&MonitorName::new("eDP-1")));
}

#[test]
fn test_display_and_as_str() {
    let ws_id = WorkspaceId::new(5);
    let mon = MonitorName::new("DP-1");
    let ws_name = WorkspaceName::new("work");

    assert_eq!(format!("{ws_id}"), "5");
    assert_eq!(format!("{mon}"), "DP-1");
    assert_eq!(format!("{ws_name}"), "work");
    assert_eq!(ws_name.as_str(), "work");
}

#[test]
fn test_monitor_operations() {
    let mut monitor = Monitor::new(MonitorName::new("DP-1"), WorkspaceId::new(1), None);
    assert_eq!(monitor.name(), &MonitorName::new("DP-1"));
    assert_eq!(*monitor.active_workspace_id(), WorkspaceId::new(1));

    monitor.set_active_workspace(WorkspaceId::new(2));
    assert_eq!(*monitor.active_workspace_id(), WorkspaceId::new(2));

    monitor.set_special_workspace(Some(WorkspaceId::new(3)));
    assert_eq!(monitor.special_workspace_id(), Some(&WorkspaceId::new(3)));
}
