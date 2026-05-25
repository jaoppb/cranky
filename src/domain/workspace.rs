use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Workspace {
    id: i32,
    monitor: String,
}

impl Workspace {
    pub fn new(id: i32, monitor: impl Into<String>) -> Self {
        Self {
            id,
            monitor: monitor.into(),
        }
    }

    pub fn id(&self) -> i32 {
        self.id
    }

    pub fn monitor(&self) -> &str {
        &self.monitor
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Monitor {
    name: String,
    active_workspace_id: i32,
    focused: bool,
}

impl Monitor {
    pub fn new(name: impl Into<String>, active_workspace_id: i32, focused: bool) -> Self {
        Self {
            name: name.into(),
            active_workspace_id,
            focused,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn active_workspace_id(&self) -> i32 {
        self.active_workspace_id
    }

    pub fn focused(&self) -> bool {
        self.focused
    }
}
