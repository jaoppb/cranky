use crate::features::workspaces::domain::{MonitorName, WorkspaceId, WorkspaceName};
use crate::shared::events::core::{WindowAddress, WindowManagerEvent, WindowTitle};

#[must_use]
pub fn parse_event(raw_line: &str) -> Option<WindowManagerEvent> {
    let (event_type, payload) = raw_line.trim().split_once(">>")?;

    match event_type {
        "workspacev2" => {
            let (id_str, name) = payload.split_once(',')?;
            Some(WindowManagerEvent::WorkspaceActivated {
                id: WorkspaceId::new(id_str.parse().ok()?),
                name: WorkspaceName::new(name),
            })
        }
        "focusedmonv2" => {
            let (mon_name, id_str) = payload.split_once(',')?;
            Some(WindowManagerEvent::MonitorFocused {
                monitor_name: MonitorName::new(mon_name),
                workspace_id: WorkspaceId::new(id_str.parse().ok()?),
            })
        }
        "createworkspacev2" => {
            let (id_str, name) = payload.split_once(',')?;
            Some(WindowManagerEvent::WorkspaceCreated {
                id: WorkspaceId::new(id_str.parse().ok()?),
                name: WorkspaceName::new(name),
            })
        }
        "destroyworkspacev2" => {
            let (id_str, name) = payload.split_once(',')?;
            Some(WindowManagerEvent::WorkspaceDestroyed {
                id: WorkspaceId::new(id_str.parse().ok()?),
                name: WorkspaceName::new(name),
            })
        }
        "moveworkspacev2" => {
            let mut parts = payload.splitn(3, ',');
            let id_str = parts.next()?;
            let name = parts.next()?;
            let monitor_name = parts.next()?;
            Some(WindowManagerEvent::WorkspaceMoved {
                id: WorkspaceId::new(id_str.parse().ok()?),
                name: WorkspaceName::new(name),
                monitor_name: MonitorName::new(monitor_name),
            })
        }
        "renameworkspace" => {
            let (id_str, name) = payload.split_once(',')?;
            Some(WindowManagerEvent::WorkspaceRenamed {
                id: WorkspaceId::new(id_str.parse().ok()?),
                new_name: WorkspaceName::new(name),
            })
        }
        "activespecialv2" => {
            let mut parts = payload.splitn(3, ',');
            let id_str = parts.next()?;
            let name_str = parts.next()?;
            let monitor_str = parts.next()?;
            let id = match id_str.parse::<i32>() {
                Ok(0) | Err(_) => None,
                Ok(val) => Some(WorkspaceId::new(val)),
            };
            let ws_name = if id.is_none() || name_str.is_empty() {
                None
            } else {
                Some(WorkspaceName::new(name_str))
            };
            Some(WindowManagerEvent::SpecialWorkspaceActivated {
                id,
                name: ws_name,
                monitor_name: MonitorName::new(monitor_str),
            })
        }
        "activewindowv2" => Some(WindowManagerEvent::ActiveWindowChanged {
            address: WindowAddress::new(payload),
        }),
        "windowtitlev2" => {
            let (address, title) = payload.split_once(',')?;
            Some(WindowManagerEvent::WindowTitleChanged {
                address: WindowAddress::new(address),
                title: WindowTitle::new(title),
            })
        }
        _ => None,
    }
}
