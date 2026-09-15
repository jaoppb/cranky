use crate::features::workspaces::adapters::event_parser::parse_event;
use crate::features::workspaces::domain::{MonitorName, WorkspaceId, WorkspaceName};
use crate::shared::events::core::{WindowAddress, WindowManagerEvent, WindowTitle};

#[test]
fn test_parse_event() {
    // workspacev2
    let e = parse_event("workspacev2>>1,test_ws").unwrap();
    assert_eq!(
        e,
        WindowManagerEvent::WorkspaceActivated {
            id: WorkspaceId::new(1),
            name: WorkspaceName::new("test_ws"),
        }
    );

    // focusedmonv2
    let e = parse_event("focusedmonv2>>DP-1,2").unwrap();
    assert_eq!(
        e,
        WindowManagerEvent::MonitorFocused {
            monitor_name: MonitorName::new("DP-1"),
            workspace_id: WorkspaceId::new(2),
        }
    );

    // createworkspacev2
    let e = parse_event("createworkspacev2>>3,new_ws").unwrap();
    assert_eq!(
        e,
        WindowManagerEvent::WorkspaceCreated {
            id: WorkspaceId::new(3),
            name: WorkspaceName::new("new_ws"),
        }
    );

    // destroyworkspacev2
    let e = parse_event("destroyworkspacev2>>4,old_ws").unwrap();
    assert_eq!(
        e,
        WindowManagerEvent::WorkspaceDestroyed {
            id: WorkspaceId::new(4),
            name: WorkspaceName::new("old_ws"),
        }
    );

    // moveworkspacev2
    let e = parse_event("moveworkspacev2>>5,moved_ws,HDMI-1").unwrap();
    assert_eq!(
        e,
        WindowManagerEvent::WorkspaceMoved {
            id: WorkspaceId::new(5),
            name: WorkspaceName::new("moved_ws"),
            monitor_name: MonitorName::new("HDMI-1"),
        }
    );

    // renameworkspace
    let e = parse_event("renameworkspace>>6,renamed_ws").unwrap();
    assert_eq!(
        e,
        WindowManagerEvent::WorkspaceRenamed {
            id: WorkspaceId::new(6),
            new_name: WorkspaceName::new("renamed_ws"),
        }
    );

    // activespecialv2
    let e = parse_event("activespecialv2>>7,special,DP-2").unwrap();
    assert_eq!(
        e,
        WindowManagerEvent::SpecialWorkspaceActivated {
            id: Some(WorkspaceId::new(7)),
            name: Some(WorkspaceName::new("special")),
            monitor_name: MonitorName::new("DP-2"),
        }
    );

    // activespecialv2 closed (0 ID)
    let e = parse_event("activespecialv2>>0,,DP-2").unwrap();
    assert_eq!(
        e,
        WindowManagerEvent::SpecialWorkspaceActivated {
            id: None,
            name: None,
            monitor_name: MonitorName::new("DP-2"),
        }
    );

    // activespecialv2 negative ID
    let e = parse_event("activespecialv2>>-98,special:magic,DP-2").unwrap();
    assert_eq!(
        e,
        WindowManagerEvent::SpecialWorkspaceActivated {
            id: Some(WorkspaceId::new(-98)),
            name: Some(WorkspaceName::new("special:magic")),
            monitor_name: MonitorName::new("DP-2"),
        }
    );

    // activewindowv2
    let e = parse_event("activewindowv2>>0x123").unwrap();
    assert_eq!(
        e,
        WindowManagerEvent::ActiveWindowChanged {
            address: WindowAddress::new("0x123"),
        }
    );

    // windowtitlev2
    let e = parse_event("windowtitlev2>>0x456,Title").unwrap();
    assert_eq!(
        e,
        WindowManagerEvent::WindowTitleChanged {
            address: WindowAddress::new("0x456"),
            title: WindowTitle::new("Title"),
        }
    );

    // invalid
    assert_eq!(parse_event("invalid>>data"), None);
    assert_eq!(parse_event("missing"), None);
}

#[test]
fn test_parse_event_monitor_removed() {
    let e =
        parse_event("monitorremovedv2>>1,HDMI-A-1,LG Electronics LG FULL HD 206AZQV5S860").unwrap();
    assert_eq!(
        e,
        WindowManagerEvent::MonitorRemoved {
            name: MonitorName::new("HDMI-A-1"),
        }
    );

    // monitoraddedv2 is intentionally not parsed (lazily inferred via MonitorFocused instead)
    assert_eq!(parse_event("monitoraddedv2>>1,HDMI-A-1,LG Display"), None);

    // monitorremovedv2 missing name field
    assert_eq!(parse_event("monitorremovedv2>>1"), None);
}
