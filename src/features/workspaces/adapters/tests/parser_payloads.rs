//! How the event parser splits and rejects payloads.
use crate::features::workspaces::adapters::event_parser::parse_event;
use crate::features::workspaces::domain::{MonitorName, WorkspaceId, WorkspaceName};
use crate::shared::events::core::{WindowAddress, WindowManagerEvent, WindowTitle};

/// Every event whose payload lacks a field, or whose workspace id is not a
/// number, parses to nothing.
#[test]
fn test_parse_event_rejects_a_malformed_payload() {
    for line in [
        "workspacev2>>1",
        "workspacev2>>x,a",
        "focusedmonv2>>DP-1",
        "focusedmonv2>>DP-1,x",
        "focusedmonv2>>DP-1,2,3",
        "createworkspacev2>>3",
        "createworkspacev2>>x,a",
        "destroyworkspacev2>>4",
        "destroyworkspacev2>>x,a",
        "moveworkspacev2>>5,a",
        "moveworkspacev2>>x,a,DP-1",
        "renameworkspace>>6",
        "renameworkspace>>x,a",
        "activespecialv2>>7,special",
        "monitorremovedv2>>",
        "monitoraddedv2>>",
        "windowtitlev2>>0x1",
    ] {
        assert_eq!(parse_event(line), None, "{line}");
    }
}

/// The last field of a payload keeps its commas, and the line is trimmed
/// before it is split.
#[test]
fn test_parse_event_keeps_commas_in_the_last_field() {
    let id = WorkspaceId::new;
    let workspace_name = WorkspaceName::new;
    for (line, expected) in [
        (
            " workspacev2>>1,a,b\n",
            WindowManagerEvent::WorkspaceActivated {
                id: id(1),
                name: workspace_name("a,b"),
            },
        ),
        (
            "createworkspacev2>>3,a,b",
            WindowManagerEvent::WorkspaceCreated {
                id: id(3),
                name: workspace_name("a,b"),
            },
        ),
        (
            "destroyworkspacev2>>4,a,b",
            WindowManagerEvent::WorkspaceDestroyed {
                id: id(4),
                name: workspace_name("a,b"),
            },
        ),
        (
            "moveworkspacev2>>5,a,DP-1,x",
            WindowManagerEvent::WorkspaceMoved {
                id: id(5),
                name: workspace_name("a"),
                monitor_name: MonitorName::new("DP-1,x"),
            },
        ),
        (
            "renameworkspace>>6,a,b",
            WindowManagerEvent::WorkspaceRenamed {
                id: id(6),
                new_name: workspace_name("a,b"),
            },
        ),
        (
            "activewindowv2>>0x1,x",
            WindowManagerEvent::ActiveWindowChanged {
                address: WindowAddress::new("0x1,x"),
            },
        ),
        (
            "windowtitlev2>>0x1,a, b",
            WindowManagerEvent::WindowTitleChanged {
                address: WindowAddress::new("0x1"),
                title: WindowTitle::new("a, b"),
            },
        ),
    ] {
        assert_eq!(parse_event(line), Some(expected), "{line}");
    }
}

/// A special workspace id that is zero or not a number closes the special
/// workspace, and so does an empty name for its name alone.
#[test]
fn test_parse_event_special_workspace_without_a_usable_id_or_name() {
    let special =
        |id: Option<i32>, name: Option<&str>| WindowManagerEvent::SpecialWorkspaceActivated {
            id: id.map(WorkspaceId::new),
            name: name.map(WorkspaceName::new),
            monitor_name: MonitorName::new("DP-2,x"),
        };
    for (line, expected) in [
        ("activespecialv2>>x,special,DP-2,x", special(None, None)),
        ("activespecialv2>>0,special,DP-2,x", special(None, None)),
        ("activespecialv2>>7,,DP-2,x", special(Some(7), None)),
        ("activespecialv2>>7,a,DP-2,x", special(Some(7), Some("a"))),
    ] {
        assert_eq!(parse_event(line), Some(expected), "{line}");
    }
}

/// A monitor added or removed is named by its second field; its id is never
/// read, and a third field is optional.
#[test]
fn test_parse_event_monitor_ids_are_not_parsed() {
    let name = MonitorName::new("HDMI-A-1");
    assert_eq!(
        (
            parse_event("monitoraddedv2>>x,HDMI-A-1"),
            parse_event("monitorremovedv2>>x,HDMI-A-1"),
        ),
        (
            Some(WindowManagerEvent::MonitorAdded { name: name.clone() }),
            Some(WindowManagerEvent::MonitorRemoved { name }),
        )
    );
}
