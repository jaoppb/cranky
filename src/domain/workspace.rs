use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Hash)]
pub struct WorkspaceId(i32);

impl WorkspaceId {
    pub fn new(id: i32) -> Self {
        Self(id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Hash)]
pub struct MonitorName(String);

impl MonitorName {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Hash)]
pub struct WorkspaceName(String);

impl WorkspaceName {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Workspace {
    id: WorkspaceId,
    name: WorkspaceName,
    monitor: MonitorName,
}

impl Workspace {
    pub fn new(id: WorkspaceId, name: WorkspaceName, monitor: MonitorName) -> Self {
        Self { id, name, monitor }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Monitor {
    name: MonitorName,
    active_workspace_id: WorkspaceId,
    special_workspace_id: Option<WorkspaceId>,
    focused: bool,
}

impl Monitor {
    pub fn new(
        name: MonitorName,
        active_workspace_id: WorkspaceId,
        special_workspace_id: Option<WorkspaceId>,
        focused: bool,
    ) -> Self {
        Self {
            name,
            active_workspace_id,
            special_workspace_id,
            focused,
        }
    }

    pub fn name(&self) -> &MonitorName {
        &self.name
    }

    pub fn focused(&self) -> bool {
        self.focused
    }
}
