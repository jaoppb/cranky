use std::collections::BTreeMap;

use crate::features::workspaces::domain::{Monitor, MonitorName, Workspace, WorkspaceId};
use crate::shared::events::core::WindowManagerEvent;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HyprlandState {
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
    pub const fn new(
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
    pub const fn monitors(&self) -> &BTreeMap<MonitorName, Monitor> {
        &self.monitors
    }

    #[must_use]
    pub const fn workspaces(&self) -> &BTreeMap<WorkspaceId, Workspace> {
        &self.workspaces
    }

    #[must_use]
    pub const fn focused_monitor(&self) -> Option<&MonitorName> {
        self.focused_monitor.as_ref()
    }

    #[must_use]
    pub fn effective_focused_monitor(&self) -> Option<MonitorName> {
        if let Some(focused) = &self.focused_monitor {
            return Some(focused.clone());
        }
        if self.monitors.len() == 1 {
            return self.monitors.keys().next().cloned();
        }
        None
    }

    pub fn apply_event(&mut self, event: &WindowManagerEvent) {
        match event {
            WindowManagerEvent::WorkspaceActivated { id, name } => {
                let mon = self.effective_focused_monitor();
                if let Some(mon_name) = mon {
                    if let Some(ws) = self.workspaces.get_mut(id) {
                        ws.set_monitor(mon_name.clone());
                    } else {
                        self.workspaces.insert(
                            id.clone(),
                            Workspace::new(id.clone(), name.clone(), Some(mon_name.clone())),
                        );
                    }
                    if let Some(m) = self.monitors.get_mut(&mon_name) {
                        m.set_active_workspace(id.clone());
                    }
                } else if !self.workspaces.contains_key(id) {
                    self.workspaces
                        .insert(id.clone(), Workspace::new(id.clone(), name.clone(), None));
                }
            }
            WindowManagerEvent::WorkspaceCreated { id, name } => {
                if !self.workspaces.contains_key(id) {
                    let mon = self.effective_focused_monitor();
                    self.workspaces
                        .insert(id.clone(), Workspace::new(id.clone(), name.clone(), mon));
                }
            }
            WindowManagerEvent::WorkspaceDestroyed { id, name: _ } => {
                self.workspaces.remove(id);
            }
            WindowManagerEvent::WorkspaceMoved {
                id,
                name,
                monitor_name,
            } => {
                if let Some(ws) = self.workspaces.get_mut(id) {
                    ws.set_monitor(monitor_name.clone());
                } else {
                    self.workspaces.insert(
                        id.clone(),
                        Workspace::new(id.clone(), name.clone(), Some(monitor_name.clone())),
                    );
                }
            }
            WindowManagerEvent::WorkspaceRenamed { id, new_name } => {
                if let Some(old) = self.workspaces.get(id) {
                    let new_ws =
                        Workspace::new(id.clone(), new_name.clone(), old.monitor().cloned());
                    self.workspaces.insert(id.clone(), new_ws);
                }
            }
            WindowManagerEvent::MonitorFocused {
                monitor_name,
                workspace_id,
            } => {
                self.focused_monitor = Some(monitor_name.clone());
                if let Some(m) = self.monitors.get_mut(monitor_name) {
                    m.set_active_workspace(workspace_id.clone());
                }

                if let Some(ws) = self.workspaces.get_mut(workspace_id) {
                    ws.set_monitor(monitor_name.clone());
                }
            }
            WindowManagerEvent::SpecialWorkspaceActivated {
                id,
                name,
                monitor_name,
            } => {
                if let Some(m) = self.monitors.get_mut(monitor_name) {
                    m.set_special_workspace(id.clone());
                }
                if let Some(ws_id) = id {
                    if let Some(ws) = self.workspaces.get_mut(ws_id) {
                        ws.set_monitor(monitor_name.clone());
                    } else if let Some(ws_name) = name {
                        self.workspaces.insert(
                            ws_id.clone(),
                            Workspace::new(
                                ws_id.clone(),
                                ws_name.clone(),
                                Some(monitor_name.clone()),
                            ),
                        );
                    }
                }
            }
            WindowManagerEvent::ActiveWindowChanged { .. }
            | WindowManagerEvent::WindowTitleChanged { .. } => {}
        }
    }
}
