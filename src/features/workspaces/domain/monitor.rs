use super::identifiers::{MonitorName, WorkspaceId};
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Monitor {
    name: MonitorName,
    active_workspace_id: WorkspaceId,
    special_workspace_id: Option<WorkspaceId>,
}

impl Monitor {
    #[must_use]
    pub const fn new(
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

    #[must_use]
    pub const fn name(&self) -> &MonitorName {
        &self.name
    }

    #[must_use]
    pub const fn active_workspace_id(&self) -> &WorkspaceId {
        &self.active_workspace_id
    }

    pub const fn set_active_workspace(&mut self, id: WorkspaceId) {
        self.active_workspace_id = id;
    }

    pub const fn set_special_workspace(&mut self, id: Option<WorkspaceId>) {
        self.special_workspace_id = id;
    }

    #[must_use]
    pub const fn special_workspace_id(&self) -> Option<&WorkspaceId> {
        self.special_workspace_id.as_ref()
    }
}
