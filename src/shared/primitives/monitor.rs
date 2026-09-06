use super::geometry::{self, Size};
use super::ids::MonitorId;
use serde::{Deserialize, Serialize};

/// Information about a monitor/display output exposed to script environments.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScriptMonitorInfo {
    id: MonitorId,
    name: String,
    size: Size,
    scale: geometry::Scale,
    is_focused: bool,
    active_workspace_id: Option<i32>,
    special_workspace_id: Option<i32>,
}

impl ScriptMonitorInfo {
    #[must_use]
    #[allow(clippy::too_many_arguments, clippy::missing_const_for_fn)]
    pub fn new(
        id: MonitorId,
        name: String,
        size: Size,
        scale: geometry::Scale,
        is_focused: bool,
        active_workspace_id: Option<i32>,
        special_workspace_id: Option<i32>,
    ) -> Self {
        Self {
            id,
            name,
            size,
            scale,
            is_focused,
            active_workspace_id,
            special_workspace_id,
        }
    }

    #[must_use]
    pub fn from_id(id: &MonitorId) -> Self {
        let s = id.as_str().to_string();
        Self {
            id: id.clone(),
            name: s,
            size: Size::new(0, 0),
            scale: geometry::Scale::new(1.0),
            is_focused: false,
            active_workspace_id: None,
            special_workspace_id: None,
        }
    }

    #[must_use]
    pub const fn id(&self) -> &MonitorId {
        &self.id
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub const fn size(&self) -> Size {
        self.size
    }

    #[must_use]
    pub const fn scale(&self) -> geometry::Scale {
        self.scale
    }

    #[must_use]
    pub const fn is_focused(&self) -> bool {
        self.is_focused
    }

    #[must_use]
    pub const fn active_workspace_id(&self) -> Option<i32> {
        self.active_workspace_id
    }

    #[must_use]
    pub const fn special_workspace_id(&self) -> Option<i32> {
        self.special_workspace_id
    }
}
