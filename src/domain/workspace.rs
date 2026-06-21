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

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Workspace {
    id: WorkspaceId,
    monitor: MonitorName,
}

impl Workspace {
    pub fn new(id: WorkspaceId, monitor: MonitorName) -> Self {
        Self { id, monitor }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Monitor {
    name: MonitorName,
    active_workspace_id: WorkspaceId,
    focused: bool,
}

impl Monitor {
    pub fn new(name: MonitorName, active_workspace_id: WorkspaceId, focused: bool) -> Self {
        Self {
            name,
            active_workspace_id,
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
