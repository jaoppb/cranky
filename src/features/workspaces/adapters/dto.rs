//! Shapes of Hyprland's JSON replies to `j/monitors` and `j/workspaces`, and their mapping into
//! domain types.
use crate::features::workspaces::domain::{
    Monitor, MonitorName, Workspace, WorkspaceId, WorkspaceName,
};
use crate::shared::primitives::MonitorFocus;
use serde::Deserialize;

#[derive(Deserialize)]
pub(super) struct HyprWorkspaceDto {
    pub(crate) id: i32,
    pub(crate) name: String,
    pub(crate) monitor: String,
}

impl HyprWorkspaceDto {
    #[must_use]
    pub(crate) fn into_domain(self) -> Workspace {
        Workspace::new(
            WorkspaceId::new(self.id),
            WorkspaceName::new(self.name),
            Some(MonitorName::new(self.monitor)),
        )
    }
}

#[derive(Deserialize)]
pub(super) struct HyprMonitorDto {
    pub(crate) name: String,
    #[serde(rename = "activeWorkspace")]
    pub(crate) active_workspace: HyprActiveWorkspaceDto,
    #[serde(rename = "specialWorkspace")]
    pub(crate) special_workspace: HyprActiveWorkspaceDto,
    /// `focused` in hyprctl's JSON, a bool.
    pub(crate) focused: MonitorFocus,
}

#[derive(Deserialize)]
pub(crate) struct HyprActiveWorkspaceDto {
    pub(crate) id: i32,
}

impl HyprMonitorDto {
    #[must_use]
    pub(crate) fn into_domain(self) -> (Monitor, MonitorFocus) {
        let special_ws_id = if self.special_workspace.id != 0 {
            Some(WorkspaceId::new(self.special_workspace.id))
        } else {
            None
        };
        let monitor = Monitor::new(
            MonitorName::new(self.name),
            WorkspaceId::new(self.active_workspace.id),
            special_ws_id,
        );
        (monitor, self.focused)
    }
}
