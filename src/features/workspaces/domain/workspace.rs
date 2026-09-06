use super::identifiers::{MonitorName, WorkspaceId, WorkspaceName};
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Workspace {
    id: WorkspaceId,
    name: WorkspaceName,
    monitor: Option<MonitorName>,
}

impl Workspace {
    #[must_use]
    pub const fn new(id: WorkspaceId, name: WorkspaceName, monitor: Option<MonitorName>) -> Self {
        Self { id, name, monitor }
    }

    #[must_use]
    pub const fn id(&self) -> &WorkspaceId {
        &self.id
    }

    #[must_use]
    pub const fn name(&self) -> &WorkspaceName {
        &self.name
    }

    #[must_use]
    pub const fn monitor(&self) -> Option<&MonitorName> {
        self.monitor.as_ref()
    }

    pub fn set_monitor(&mut self, monitor: MonitorName) {
        self.monitor = Some(monitor);
    }
}
