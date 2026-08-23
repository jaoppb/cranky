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

    pub fn special_workspace_id(&self) -> Option<&WorkspaceId> {
        self.special_workspace_id.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workspace_id() {
        let id = WorkspaceId::new(42);
        assert_eq!(id, WorkspaceId(42));
    }

    #[test]
    fn test_monitor_name() {
        let name = MonitorName::new("DP-1");
        assert_eq!(name.as_str(), "DP-1");
    }

    #[test]
    fn test_workspace_name() {
        let name = WorkspaceName::new("1");
        assert_eq!(name, WorkspaceName("1".to_string()));
    }

    #[test]
    fn test_workspace_operations() {
        let mut ws = Workspace::new(WorkspaceId::new(1), WorkspaceName::new("1"), None);
        assert_eq!(*ws.id(), WorkspaceId::new(1));
        assert_eq!(ws.monitor(), None);

        ws.set_monitor(MonitorName::new("eDP-1"));
        assert_eq!(ws.monitor(), Some(&MonitorName::new("eDP-1")));
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
}
