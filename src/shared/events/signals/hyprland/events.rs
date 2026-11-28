//! How each window-manager event changes the `HyprlandState`.

use std::collections::btree_map::Entry;

use crate::features::workspaces::domain::{
    Monitor, MonitorName, Workspace, WorkspaceId, WorkspaceName,
};
use crate::shared::events::core::WindowManagerEvent as Event;

use super::HyprlandState;

impl HyprlandState {
    pub(crate) fn apply_event(&mut self, event: &Event) {
        match event {
            Event::WorkspaceActivated { id, name } => self.apply_workspace_activated(id, name),
            Event::WorkspaceCreated { id, name } => self.create_workspace(id, name),
            Event::WorkspaceDestroyed { id, name: _ } => self.destroy_workspace(id),
            Event::WorkspaceMoved {
                id,
                name,
                monitor_name,
            } => place_workspace(self.workspaces.entry(id.clone()), Some(name), monitor_name),
            Event::WorkspaceRenamed { id, new_name } => self.rename_workspace(id, new_name),
            Event::MonitorFocused {
                monitor_name,
                workspace_id,
            } => self.focus_monitor(monitor_name, workspace_id),
            Event::SpecialWorkspaceActivated {
                id,
                name,
                monitor_name,
            } => self.show_special(ShownSpecial::new(id.as_ref(), name.as_ref()), monitor_name),
            Event::MonitorRemoved { name } => self.remove_monitor(name),
            // MonitorAdded carries no active-workspace id, so it can't be applied
            // incrementally - signal_loop treats it as an unconditional resync trigger.
            Event::MonitorAdded { .. }
            | Event::ActiveWindowChanged { .. }
            | Event::WindowTitleChanged { .. } => {}
        }
    }

    fn apply_workspace_activated(&mut self, id: &WorkspaceId, name: &WorkspaceName) {
        let Some(mon_name) = self.effective_focused_monitor() else {
            self.workspaces
                .entry(id.clone())
                .or_insert_with(|| Workspace::new(id.clone(), name.clone(), None));
            return;
        };
        place_workspace(self.workspaces.entry(id.clone()), Some(name), &mon_name);
        if let Some(monitor) = self.monitors.get_mut(&mon_name) {
            monitor.set_active_workspace(id.clone());
        }
    }

    /// Records a new workspace on the focused monitor; one already known is left as it is.
    fn create_workspace(&mut self, id: &WorkspaceId, name: &WorkspaceName) {
        if !self.workspaces.contains_key(id) {
            let mon = self.effective_focused_monitor();
            self.workspaces
                .insert(id.clone(), Workspace::new(id.clone(), name.clone(), mon));
        }
    }

    /// Renames a known workspace, keeping its monitor; an unknown one is ignored.
    fn rename_workspace(&mut self, id: &WorkspaceId, new_name: &WorkspaceName) {
        if let Some(old) = self.workspaces.get(id) {
            let new_ws = Workspace::new(id.clone(), new_name.clone(), old.monitor().cloned());
            self.workspaces.insert(id.clone(), new_ws);
        }
    }

    /// Focuses `monitor_name` showing `workspace_id`, recording the monitor if it is new and
    /// moving the workspace onto it if it is known.
    fn focus_monitor(&mut self, monitor_name: &MonitorName, workspace_id: &WorkspaceId) {
        self.focused_monitor = Some(monitor_name.clone());
        if let Some(monitor) = self.monitors.get_mut(monitor_name) {
            monitor.set_active_workspace(workspace_id.clone());
        } else {
            self.monitors.insert(
                monitor_name.clone(),
                Monitor::new(monitor_name.clone(), workspace_id.clone(), None),
            );
        }

        if let Some(workspace) = self.workspaces.get_mut(workspace_id) {
            workspace.set_monitor(monitor_name.clone());
        }
    }

    /// Forgets the workspace `id`.
    fn destroy_workspace(&mut self, id: &WorkspaceId) {
        self.workspaces.remove(id);
    }

    /// Shows `special` in `monitor_name`'s special slot, then moves the special workspace to that
    /// monitor, recording it if it is new and named.
    fn show_special(&mut self, special: ShownSpecial<'_>, monitor_name: &MonitorName) {
        if let Some(monitor) = self.monitors.get_mut(monitor_name) {
            monitor.set_special_workspace(special.id.cloned());
        }
        if let Some(ws_id) = special.id {
            let slot = self.workspaces.entry(ws_id.clone());
            place_workspace(slot, special.name, monitor_name);
        }
    }

    /// Forgets `name`, and the focus with it if it was the focused monitor.
    fn remove_monitor(&mut self, name: &MonitorName) {
        self.monitors.remove(name);
        if self.focused_monitor.as_ref() == Some(name) {
            self.focused_monitor = None;
        }
    }
}

/// The special workspace an event shows: its id, `None` when the special workspace closes, and
/// the name to record it under if it is new.
#[derive(Clone, Copy)]
struct ShownSpecial<'event> {
    id: Option<&'event WorkspaceId>,
    name: Option<&'event WorkspaceName>,
}

impl<'event> ShownSpecial<'event> {
    const fn new(id: Option<&'event WorkspaceId>, name: Option<&'event WorkspaceName>) -> Self {
        Self { id, name }
    }
}

/// Moves the workspace in `slot` to `monitor_name`, recording an unknown one if it has a name.
fn place_workspace(
    slot: Entry<'_, WorkspaceId, Workspace>,
    name: Option<&WorkspaceName>,
    monitor_name: &MonitorName,
) {
    match slot {
        Entry::Occupied(mut known) => known.get_mut().set_monitor(monitor_name.clone()),
        Entry::Vacant(unknown) => {
            if let Some(ws_name) = name {
                let (id, monitor) = (unknown.key().clone(), Some(monitor_name.clone()));
                unknown.insert(Workspace::new(id, ws_name.clone(), monitor));
            }
        }
    }
}
