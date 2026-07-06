use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Hash)]
pub struct WorkspaceId(i32);

impl WorkspaceId {
    pub fn new(id: i32) -> Self {
        Self(id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Hash)]
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
    monitor: Option<MonitorName>,
}

impl Workspace {
    pub fn new(id: WorkspaceId, name: WorkspaceName, monitor: Option<MonitorName>) -> Self {
        Self { id, name, monitor }
    }
    
    pub fn id(&self) -> &WorkspaceId {
        &self.id
    }
    
    pub fn monitor(&self) -> Option<&MonitorName> {
        self.monitor.as_ref()
    }
    
    pub fn set_monitor(&mut self, monitor: MonitorName) {
        self.monitor = Some(monitor);
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Monitor {
    name: MonitorName,
    active_workspace_id: WorkspaceId,
    special_workspace_id: Option<WorkspaceId>,
}

impl Monitor {
    pub fn new(
        name: MonitorName,
        active_workspace_id: WorkspaceId,
        special_workspace_id: Option<WorkspaceId>,
    ) -> Self {
        Self {
            name,
            active_workspace_id,
            special_workspace_id,
        }
    }

    pub fn name(&self) -> &MonitorName {
        &self.name
    }
    
    pub fn active_workspace_id(&self) -> &WorkspaceId {
        &self.active_workspace_id
    }
    
    pub fn set_active_workspace(&mut self, id: WorkspaceId) {
        self.active_workspace_id = id;
    }
    
    pub fn set_special_workspace(&mut self, id: Option<WorkspaceId>) {
        self.special_workspace_id = id;
    }
}
