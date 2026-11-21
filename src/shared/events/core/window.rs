//! Window-manager events (workspaces, monitors, windows) and their identifier types.

use crate::features::workspaces::domain::{MonitorName, WorkspaceId, WorkspaceName};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct WindowAddress(String);
impl WindowAddress {
    #[must_use]
    pub(crate) fn new(addr: impl Into<String>) -> Self {
        Self(addr.into())
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct WindowTitle(String);
impl WindowTitle {
    #[must_use]
    pub(crate) fn new(title: impl Into<String>) -> Self {
        Self(title.into())
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum WindowManagerEvent {
    WorkspaceActivated {
        id: WorkspaceId,
        name: WorkspaceName,
    },
    MonitorFocused {
        monitor_name: MonitorName,
        workspace_id: WorkspaceId,
    },
    WorkspaceCreated {
        id: WorkspaceId,
        name: WorkspaceName,
    },
    WorkspaceDestroyed {
        id: WorkspaceId,
        name: WorkspaceName,
    },
    WorkspaceMoved {
        id: WorkspaceId,
        name: WorkspaceName,
        monitor_name: MonitorName,
    },
    WorkspaceRenamed {
        id: WorkspaceId,
        new_name: WorkspaceName,
    },
    SpecialWorkspaceActivated {
        id: Option<WorkspaceId>,
        name: Option<WorkspaceName>,
        monitor_name: MonitorName,
    },
    MonitorRemoved {
        name: MonitorName,
    },
    MonitorAdded {
        name: MonitorName,
    },
    ActiveWindowChanged {
        address: WindowAddress,
    },
    WindowTitleChanged {
        address: WindowAddress,
        title: WindowTitle,
    },
}
