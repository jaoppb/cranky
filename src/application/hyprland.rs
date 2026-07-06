use crate::domain::hyprland::events::HyprlandEvent;
use crate::domain::signals::HyprlandState;
use crate::domain::workspace::Workspace;

pub struct HyprlandStateUpdater;

impl HyprlandStateUpdater {
    pub fn apply_event(state: &mut HyprlandState, event: HyprlandEvent) {
        match event {
            HyprlandEvent::Workspace(id, _ws_name) => {
                if let Some(mon) = state.monitors.iter_mut().find(|m| m.focused()) {
                    mon.set_active_workspace_id(id);
                }
            }
            HyprlandEvent::FocusedMon(mon_name, ws_name) => {
                for mon in &mut state.monitors {
                    mon.set_focused(mon.name() == &mon_name);
                }
                if let Some(ws) = state.workspaces.iter_mut().find(|w| w.name() == &ws_name) {
                    let ws_id = ws.id().clone();
                    ws.set_monitor(Some(mon_name.clone()));
                    if let Some(mon) = state.monitors.iter_mut().find(|m| m.name() == &mon_name) {
                        mon.set_active_workspace_id(ws_id);
                    }
                }
            }
            HyprlandEvent::CreateWorkspace(id, ws_name) => {
                state.workspaces.push(Workspace::new(id, ws_name, None));
            }
            HyprlandEvent::DestroyWorkspace(id, _ws_name) => {
                state.workspaces.retain(|w| w.id() != &id);
            }
            HyprlandEvent::MoveWorkspace(id, _ws_name, mon_name) => {
                if let Some(ws) = state.workspaces.iter_mut().find(|w| w.id() == &id) {
                    ws.set_monitor(Some(mon_name));
                }
            }
            _ => {}
        }
    }
}
