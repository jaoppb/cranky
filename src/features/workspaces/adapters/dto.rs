use crate::features::workspaces::domain::{
    Monitor, MonitorName, Workspace, WorkspaceId, WorkspaceName,
};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct HyprWorkspaceDto {
    pub id: i32,
    pub name: String,
    pub monitor: String,
}

impl HyprWorkspaceDto {
    #[must_use]
    pub fn into_domain(self) -> Workspace {
        Workspace::new(
            WorkspaceId::new(self.id),
            WorkspaceName::new(self.name),
            Some(MonitorName::new(self.monitor)),
        )
    }
}

#[derive(Deserialize)]
pub struct HyprMonitorDto {
    pub name: String,
    #[serde(rename = "activeWorkspace")]
    pub active_workspace: HyprActiveWorkspaceDto,
    #[serde(rename = "specialWorkspace")]
    pub special_workspace: HyprActiveWorkspaceDto,
    pub focused: bool,
}

#[derive(Deserialize)]
pub struct HyprActiveWorkspaceDto {
    pub id: i32,
}

impl HyprMonitorDto {
    #[must_use]
    pub fn into_domain(self) -> (Monitor, bool) {
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
