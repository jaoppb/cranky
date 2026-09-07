use crate::shared::primitives::geometry::Position;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize)]
pub enum SurfaceKind {
    #[default]
    Bar,
    Popup,
    Panel,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PointerEvent {
    PointerEnter {
        surface: SurfaceKind,
    },
    PointerLeave {
        surface: SurfaceKind,
    },
    PointerMotion {
        surface: SurfaceKind,
        pos: Position,
    },
    ButtonPress {
        surface: SurfaceKind,
        button: PointerButton,
        pos: Position,
    },
    ButtonRelease {
        surface: SurfaceKind,
        button: PointerButton,
        pos: Position,
    },
    Click {
        surface: SurfaceKind,
        button: PointerButton,
        pos: Position,
    },
    Scroll {
        surface: SurfaceKind,
        axis: ScrollAxis,
        amount: ScrollDelta,
    },
}

impl PointerEvent {
    #[must_use]
    pub const fn surface(&self) -> SurfaceKind {
        match self {
            Self::PointerEnter { surface }
            | Self::PointerLeave { surface }
            | Self::PointerMotion { surface, .. }
            | Self::ButtonPress { surface, .. }
            | Self::ButtonRelease { surface, .. }
            | Self::Click { surface, .. }
            | Self::Scroll { surface, .. } => *surface,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SurfaceLifecycleEvent {
    PopupDismissed,
    PanelDismissed,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InteractionEvent {
    Pointer(PointerEvent),
    Lifecycle(SurfaceLifecycleEvent),
}

pub type PointerSender = tokio::sync::broadcast::Sender<(
    crate::shared::primitives::ModuleId,
    crate::shared::primitives::MonitorId,
    InteractionEvent,
)>;
pub type PointerReceiver = tokio::sync::broadcast::Receiver<(
    crate::shared::primitives::ModuleId,
    crate::shared::primitives::MonitorId,
    InteractionEvent,
)>;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub struct PointerSerial(u32);

impl PointerSerial {
    #[must_use]
    pub const fn new(serial: u32) -> Self {
        Self(serial)
    }

    #[must_use]
    pub const fn value(&self) -> u32 {
        self.0
    }
}

impl std::fmt::Display for PointerSerial {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

