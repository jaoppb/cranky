use crate::features::workspaces::adapters::event_parser::parse_event;
use crate::features::workspaces::domain::{MonitorName, WorkspaceId, WorkspaceName};
use crate::shared::events::core::{WindowAddress, WindowManagerEvent, WindowTitle};

/// Checks that `line` parses into `expected`.
pub(super) fn assert_parses(line: &str, expected: WindowManagerEvent) {
    assert_eq!(parse_event(line), Some(expected), "{line}");
}

#[test]
fn test_parse_event() {
    // workspacev2
    assert_parses(
        "workspacev2>>1,test_ws",
        WindowManagerEvent::WorkspaceActivated {
            id: WorkspaceId::new(1),
            name: WorkspaceName::new("test_ws"),
        },
    );

    // focusedmonv2
    assert_parses(
        "focusedmonv2>>DP-1,2",
        WindowManagerEvent::MonitorFocused {
            monitor_name: MonitorName::new("DP-1"),
            workspace_id: WorkspaceId::new(2),
        },
    );

    // createworkspacev2
    assert_parses(
        "createworkspacev2>>3,new_ws",
        WindowManagerEvent::WorkspaceCreated {
            id: WorkspaceId::new(3),
            name: WorkspaceName::new("new_ws"),
        },
    );
}

#[test]
fn test_parse_event_destroyed_moved_and_renamed_workspaces() {
    // destroyworkspacev2
    assert_parses(
        "destroyworkspacev2>>4,old_ws",
        WindowManagerEvent::WorkspaceDestroyed {
            id: WorkspaceId::new(4),
            name: WorkspaceName::new("old_ws"),
        },
    );

    // moveworkspacev2
    assert_parses(
        "moveworkspacev2>>5,moved_ws,HDMI-1",
        WindowManagerEvent::WorkspaceMoved {
            id: WorkspaceId::new(5),
            name: WorkspaceName::new("moved_ws"),
            monitor_name: MonitorName::new("HDMI-1"),
        },
    );

    // renameworkspace
    assert_parses(
        "renameworkspace>>6,renamed_ws",
        WindowManagerEvent::WorkspaceRenamed {
            id: WorkspaceId::new(6),
            new_name: WorkspaceName::new("renamed_ws"),
        },
    );
}

#[test]
fn test_parse_event_special_workspace() {
    let special =
        |id: Option<i32>, name: Option<&str>| WindowManagerEvent::SpecialWorkspaceActivated {
            id: id.map(WorkspaceId::new),
            name: name.map(WorkspaceName::new),
            monitor_name: MonitorName::new("DP-2"),
        };

    // activespecialv2
    assert_parses(
        "activespecialv2>>7,special,DP-2",
        special(Some(7), Some("special")),
    );

    // activespecialv2 closed (0 ID)
    assert_parses("activespecialv2>>0,,DP-2", special(None, None));

    // activespecialv2 negative ID
    assert_parses(
        "activespecialv2>>-98,special:magic,DP-2",
        special(Some(-98), Some("special:magic")),
    );
}

#[test]
fn test_parse_event_windows_and_invalid_lines() {
    // activewindowv2
    assert_parses(
        "activewindowv2>>0x123",
        WindowManagerEvent::ActiveWindowChanged {
            address: WindowAddress::new("0x123"),
        },
    );

    // windowtitlev2
    assert_parses(
        "windowtitlev2>>0x456,Title",
        WindowManagerEvent::WindowTitleChanged {
            address: WindowAddress::new("0x456"),
            title: WindowTitle::new("Title"),
        },
    );

    // invalid
    assert_eq!(parse_event("invalid>>data"), None);
    assert_eq!(parse_event("missing"), None);
}

#[test]
fn test_parse_event_monitor_removed() {
    let event =
        parse_event("monitorremovedv2>>1,HDMI-A-1,LG Electronics LG FULL HD 206AZQV5S860").unwrap();
    assert_eq!(
        event,
        WindowManagerEvent::MonitorRemoved {
            name: MonitorName::new("HDMI-A-1"),
        }
    );

    // monitorremovedv2 missing name field
    assert_eq!(parse_event("monitorremovedv2>>1"), None);
}

#[test]
fn test_parse_event_monitor_added() {
    let event = parse_event("monitoraddedv2>>1,HDMI-A-1,LG Display").unwrap();
    assert_eq!(
        event,
        WindowManagerEvent::MonitorAdded {
            name: MonitorName::new("HDMI-A-1"),
        }
    );

    // monitoraddedv2 missing name field
    assert_eq!(parse_event("monitoraddedv2>>1"), None);
}
