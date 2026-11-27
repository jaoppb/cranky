//! Builders for the workspaces, monitors and states the adapter tests check.
use crate::features::workspaces::domain::{
    Monitor, MonitorName, Workspace, WorkspaceId, WorkspaceName,
};
use crate::shared::events::signals::HyprlandState;

/// Workspace `id`, named `name`, on `monitor` if it has one.
pub(in crate::features::workspaces::adapters) fn workspace(
    id: i32,
    name: &str,
    monitor: Option<&str>,
) -> Workspace {
    Workspace::new(
        WorkspaceId::new(id),
        WorkspaceName::new(name),
        monitor.map(MonitorName::new),
    )
}

/// Monitor `name`, showing workspace `active` and special workspace `special` if it has one.
pub(in crate::features::workspaces::adapters) fn monitor(
    name: &str,
    active: i32,
    special: Option<i32>,
) -> Monitor {
    Monitor::new(
        MonitorName::new(name),
        WorkspaceId::new(active),
        special.map(WorkspaceId::new),
    )
}

/// A state holding `workspaces` keyed by id and `monitors` keyed by name, with `focused`
/// focused.
pub(in crate::features::workspaces::adapters) fn state(
    workspaces: Vec<Workspace>,
    monitors: Vec<Monitor>,
    focused: Option<&str>,
) -> HyprlandState {
    HyprlandState::new(
        workspaces
            .into_iter()
            .map(|workspace| (workspace.id().clone(), workspace))
            .collect(),
        monitors
            .into_iter()
            .map(|monitor| (monitor.name().clone(), monitor))
            .collect(),
        focused.map(MonitorName::new),
    )
}
