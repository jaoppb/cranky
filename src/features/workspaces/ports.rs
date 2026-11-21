use thiserror::Error;

#[derive(Error, Debug)]
pub(super) enum WindowManagerError {
    #[error("Window manager IPC error: {reason}")]
    IpcError { reason: String },
}

pub(super) type WindowManagerState = (
    std::collections::BTreeMap<
        crate::features::workspaces::domain::WorkspaceId,
        crate::features::workspaces::domain::Workspace,
    >,
    std::collections::BTreeMap<
        crate::features::workspaces::domain::MonitorName,
        crate::features::workspaces::domain::Monitor,
    >,
    Option<crate::features::workspaces::domain::MonitorName>,
);

pub(super) trait WindowManagerPort: Send + Sync {
    /// Retrieves the current state of the window manager.
    ///
    /// # Errors
    ///
    /// Returns [`WindowManagerError`] if querying the window manager fails.
    fn get_state(&self) -> Result<WindowManagerState, WindowManagerError>;
}
