use crate::features::workspaces::domain::{MonitorName, WorkspaceId, WorkspaceName};
use crate::shared::events::core::{WindowAddress, WindowManagerEvent, WindowTitle};

#[must_use]
pub(super) fn parse_event(raw_line: &str) -> Option<WindowManagerEvent> {
    let (event_type, payload) = raw_line.trim().split_once(">>")?;

    match event_type {
        "workspacev2" => workspace_activated(payload),
        "focusedmonv2" => monitor_focused(payload),
        "createworkspacev2" => workspace_created(payload),
        "destroyworkspacev2" => workspace_destroyed(payload),
        "moveworkspacev2" => workspace_moved(payload),
        "renameworkspace" => workspace_renamed(payload),
        "activespecialv2" => special_workspace_activated(payload),
        "monitorremovedv2" => monitor_removed(payload),
        "monitoraddedv2" => monitor_added(payload),
        "activewindowv2" => Some(WindowManagerEvent::ActiveWindowChanged {
            address: WindowAddress::new(payload),
        }),
        "windowtitlev2" => window_title_changed(payload),
        _ => None,
    }
}

/// Splits an `id,name` payload into a numeric workspace id and the name, which keeps its commas.
fn id_and_name(payload: &str) -> Option<(WorkspaceId, WorkspaceName)> {
    let (id_str, name) = payload.split_once(',')?;
    Some((
        WorkspaceId::new(id_str.parse().ok()?),
        WorkspaceName::new(name),
    ))
}

/// Splits a payload into its first three fields, the third keeping any further commas.
fn three_fields(payload: &str) -> Option<(&str, &str, &str)> {
    let mut parts = payload.splitn(3, ',');
    Some((parts.next()?, parts.next()?, parts.next()?))
}

/// The monitor name a `monitoraddedv2`/`monitorremovedv2` payload carries in its second field;
/// the id before it is never read.
fn monitor_name_field(payload: &str) -> Option<MonitorName> {
    let mut parts = payload.splitn(3, ',');
    let _id = parts.next()?;
    Some(MonitorName::new(parts.next()?))
}

/// Parses `workspacev2>>id,name`.
fn workspace_activated(payload: &str) -> Option<WindowManagerEvent> {
    let (id, name) = id_and_name(payload)?;
    Some(WindowManagerEvent::WorkspaceActivated { id, name })
}

/// Parses `focusedmonv2>>monitor,id`.
fn monitor_focused(payload: &str) -> Option<WindowManagerEvent> {
    let (mon_name, id_str) = payload.split_once(',')?;
    Some(WindowManagerEvent::MonitorFocused {
        monitor_name: MonitorName::new(mon_name),
        workspace_id: WorkspaceId::new(id_str.parse().ok()?),
    })
}

/// Parses `createworkspacev2>>id,name`.
fn workspace_created(payload: &str) -> Option<WindowManagerEvent> {
    let (id, name) = id_and_name(payload)?;
    Some(WindowManagerEvent::WorkspaceCreated { id, name })
}

/// Parses `destroyworkspacev2>>id,name`.
fn workspace_destroyed(payload: &str) -> Option<WindowManagerEvent> {
    let (id, name) = id_and_name(payload)?;
    Some(WindowManagerEvent::WorkspaceDestroyed { id, name })
}

/// Parses `moveworkspacev2>>id,name,monitor`.
fn workspace_moved(payload: &str) -> Option<WindowManagerEvent> {
    let (id_str, name, monitor_name) = three_fields(payload)?;
    Some(WindowManagerEvent::WorkspaceMoved {
        id: WorkspaceId::new(id_str.parse().ok()?),
        name: WorkspaceName::new(name),
        monitor_name: MonitorName::new(monitor_name),
    })
}

/// Parses `renameworkspace>>id,new_name`.
fn workspace_renamed(payload: &str) -> Option<WindowManagerEvent> {
    let (id, new_name) = id_and_name(payload)?;
    Some(WindowManagerEvent::WorkspaceRenamed { id, new_name })
}

/// Parses `activespecialv2>>id,name,monitor`: an id that is zero or not a number closes the
/// special workspace, and the name is kept only with an id and when it is not empty.
fn special_workspace_activated(payload: &str) -> Option<WindowManagerEvent> {
    let (id_str, name_str, monitor_str) = three_fields(payload)?;
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

/// Parses `monitorremovedv2>>id,name,description`.
fn monitor_removed(payload: &str) -> Option<WindowManagerEvent> {
    let name = monitor_name_field(payload)?;
    Some(WindowManagerEvent::MonitorRemoved { name })
}

/// Parses `monitoraddedv2>>id,name,description`.
fn monitor_added(payload: &str) -> Option<WindowManagerEvent> {
    let name = monitor_name_field(payload)?;
    Some(WindowManagerEvent::MonitorAdded { name })
}

/// Parses `windowtitlev2>>address,title`.
fn window_title_changed(payload: &str) -> Option<WindowManagerEvent> {
    let (address, title) = payload.split_once(',')?;
    Some(WindowManagerEvent::WindowTitleChanged {
        address: WindowAddress::new(address),
        title: WindowTitle::new(title),
    })
}
