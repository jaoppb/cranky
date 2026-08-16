use thiserror::Error;

#[derive(Error, Debug)]
pub enum WindowManagerError {
    #[error("Window manager IPC error: {reason}")]
    IpcError { reason: String },
}

pub type WindowManagerState = (
    std::collections::BTreeMap<crate::features::workspaces::domain::WorkspaceId, crate::features::workspaces::domain::Workspace>,
    std::collections::BTreeMap<crate::features::workspaces::domain::MonitorName, crate::features::workspaces::domain::Monitor>,
    Option<crate::features::workspaces::domain::MonitorName>,
);

pub trait WindowManagerPort: Send + Sync {
    fn get_state(&self) -> Result<WindowManagerState, WindowManagerError>;
}
