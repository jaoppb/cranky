use crate::features::workspaces::domain::{MonitorName, WorkspaceId, WorkspaceName};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WindowAddress(String);
impl WindowAddress {
    #[must_use]
    pub fn new(addr: impl Into<String>) -> Self {
        Self(addr.into())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WindowTitle(String);
impl WindowTitle {
    #[must_use]
    pub fn new(title: impl Into<String>) -> Self {
        Self(title.into())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WindowManagerEvent {
    WorkspaceActivated {
        id: WorkspaceId,
        name: WorkspaceName,
    },
    MonitorFocused {
        monitor_name: MonitorName,
        workspace_id: WorkspaceId,
    },
    WorkspaceCreated {
        id: WorkspaceId,
        name: WorkspaceName,
    },
    WorkspaceDestroyed {
        id: WorkspaceId,
        name: WorkspaceName,
    },
    WorkspaceMoved {
        id: WorkspaceId,
        name: WorkspaceName,
        monitor_name: MonitorName,
    },
    WorkspaceRenamed {
        id: WorkspaceId,
        new_name: WorkspaceName,
    },
    SpecialWorkspaceActivated {
        id: Option<WorkspaceId>,
        name: Option<WorkspaceName>,
        monitor_name: MonitorName,
    },
    ActiveWindowChanged {
        address: WindowAddress,
    },
    WindowTitleChanged {
        address: WindowAddress,
        title: WindowTitle,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum PointerButton {
    Left,
    Middle,
    Right,
    Side,
    Extra,
    Forward,
    Back,
    Other(u32),
}

impl PointerButton {
    #[must_use]
    pub const fn from_raw(button: u32) -> Self {
        match button {
            0x110 => Self::Left,
            0x111 => Self::Right,
            0x112 => Self::Middle,
            0x113 => Self::Side,
            0x114 => Self::Extra,
            0x115 => Self::Forward,
            0x116 => Self::Back,
            other => Self::Other(other),
        }
    }

    #[must_use]
    pub const fn to_raw(self) -> u32 {
        match self {
            Self::Left => 0x110,
            Self::Right => 0x111,
            Self::Middle => 0x112,
            Self::Side => 0x113,
            Self::Extra => 0x114,
            Self::Forward => 0x115,
            Self::Back => 0x116,
            Self::Other(raw) => raw,
        }
    }

    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_ascii_lowercase().as_str() {
            "left" => Some(Self::Left),
            "right" => Some(Self::Right),
            "middle" => Some(Self::Middle),
            "side" => Some(Self::Side),
            "extra" => Some(Self::Extra),
            "forward" => Some(Self::Forward),
            "back" => Some(Self::Back),
            s => s.parse::<u32>().ok().map(Self::from_raw).or_else(|| {
                s.strip_prefix("0x")
                    .and_then(|hex| u32::from_str_radix(hex, 16).ok())
                    .map(Self::from_raw)
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScrollAxis {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct ScrollDelta(f64);

impl ScrollDelta {
    #[must_use]
    pub const fn new(value: f64) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn value(&self) -> f64 {
        self.0
    }
}

use crate::shared::primitives::geometry::Position;

#[derive(Debug, Clone, PartialEq)]
pub enum PointerEvent {
    PointerEnter,
    PointerLeave,
    PointerMotion {
        pos: Position,
    },
    ButtonPress {
        button: PointerButton,
        pos: Position,
    },
    ButtonRelease {
        button: PointerButton,
        pos: Position,
    },
    Click {
        button: PointerButton,
        pos: Position,
    },
    Scroll {
        axis: ScrollAxis,
        amount: ScrollDelta,
    },
    PopupDismissed,
}

pub type PointerSender = tokio::sync::broadcast::Sender<(
    crate::shared::primitives::ModuleId,
    crate::shared::primitives::MonitorId,
    PointerEvent,
)>;
pub type PointerReceiver = tokio::sync::broadcast::Receiver<(
    crate::shared::primitives::ModuleId,
    crate::shared::primitives::MonitorId,
    PointerEvent,
)>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_window_address() {
        let addr = WindowAddress::new("0x1234");
        assert_eq!(addr, WindowAddress("0x1234".to_string()));
    }

    #[test]
    fn test_window_title() {
        let title = WindowTitle::new("Firefox");
        assert_eq!(title, WindowTitle("Firefox".to_string()));
    }

    #[test]
    fn test_pointer_button_conversion() {
        assert_eq!(PointerButton::from_raw(0x110), PointerButton::Left);
        assert_eq!(PointerButton::from_raw(0x111), PointerButton::Right);
        assert_eq!(PointerButton::from_raw(0x112), PointerButton::Middle);
        assert_eq!(PointerButton::from_raw(0x113), PointerButton::Side);
        assert_eq!(PointerButton::from_raw(0x114), PointerButton::Extra);
        assert_eq!(PointerButton::from_raw(0x115), PointerButton::Forward);
        assert_eq!(PointerButton::from_raw(0x116), PointerButton::Back);
        assert_eq!(PointerButton::from_raw(999), PointerButton::Other(999));

        assert_eq!(PointerButton::Left.to_raw(), 0x110);
        assert_eq!(PointerButton::Right.to_raw(), 0x111);
        assert_eq!(PointerButton::Middle.to_raw(), 0x112);
        assert_eq!(PointerButton::Side.to_raw(), 0x113);
        assert_eq!(PointerButton::Extra.to_raw(), 0x114);
        assert_eq!(PointerButton::Forward.to_raw(), 0x115);
        assert_eq!(PointerButton::Back.to_raw(), 0x116);
        assert_eq!(PointerButton::Other(999).to_raw(), 999);

        assert_eq!(PointerButton::from_name("left"), Some(PointerButton::Left));
        assert_eq!(
            PointerButton::from_name("RIGHT"),
            Some(PointerButton::Right)
        );
        assert_eq!(
            PointerButton::from_name("middle"),
            Some(PointerButton::Middle)
        );
        assert_eq!(PointerButton::from_name("side"), Some(PointerButton::Side));
        assert_eq!(
            PointerButton::from_name("extra"),
            Some(PointerButton::Extra)
        );
        assert_eq!(
            PointerButton::from_name("forward"),
            Some(PointerButton::Forward)
        );
        assert_eq!(PointerButton::from_name("back"), Some(PointerButton::Back));
        assert_eq!(PointerButton::from_name("275"), Some(PointerButton::Side));
        assert_eq!(PointerButton::from_name("0x113"), Some(PointerButton::Side));
        assert_eq!(
            PointerButton::from_name("999"),
            Some(PointerButton::Other(999))
        );
        assert_eq!(PointerButton::from_name("unknown_btn"), None);
    }

    #[test]
    fn test_scroll_delta() {
        let delta = ScrollDelta::new(15.5);
        assert!((delta.value() - 15.5).abs() < f64::EPSILON);
    }
}
