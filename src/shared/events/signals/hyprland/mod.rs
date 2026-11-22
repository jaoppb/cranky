//! Hyprland workspace, monitor and focus state, updated by applying window-manager events.

mod events;

use std::collections::BTreeMap;

use crate::features::workspaces::domain::{Monitor, MonitorName, Workspace, WorkspaceId};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct HyprlandState {
    workspaces: BTreeMap<WorkspaceId, Workspace>,
    monitors: BTreeMap<MonitorName, Monitor>,
    focused_monitor: Option<MonitorName>,
}

impl serde::Serialize for HyprlandState {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("HyprlandState", 3)?;

        let workspaces_vec: Vec<_> = self.workspaces.values().collect();
        state.serialize_field("workspaces", &workspaces_vec)?;

        let monitors_vec: Vec<_> = self.monitors.values().collect();
        state.serialize_field("monitors", &monitors_vec)?;

        state.serialize_field("focused_monitor", &self.focused_monitor)?;
        state.end()
    }
}

impl HyprlandState {
    #[must_use]
    pub(crate) const fn new(
        workspaces: BTreeMap<WorkspaceId, Workspace>,
        monitors: BTreeMap<MonitorName, Monitor>,
        focused_monitor: Option<MonitorName>,
    ) -> Self {
        Self {
            workspaces,
            monitors,
            focused_monitor,
        }
    }

    #[must_use]
    pub(crate) const fn monitors(&self) -> &BTreeMap<MonitorName, Monitor> {
        &self.monitors
    }

    #[must_use]
    pub(crate) const fn workspaces(&self) -> &BTreeMap<WorkspaceId, Workspace> {
        &self.workspaces
    }

    #[must_use]
    pub(crate) const fn focused_monitor(&self) -> Option<&MonitorName> {
        self.focused_monitor.as_ref()
    }

    #[must_use]
    pub(crate) fn effective_focused_monitor(&self) -> Option<MonitorName> {
        if let Some(focused) = &self.focused_monitor {
            return Some(focused.clone());
        }
        if self.monitors.len() == 1 {
            return self.monitors.keys().next().cloned();
        }
        None
    }
}
