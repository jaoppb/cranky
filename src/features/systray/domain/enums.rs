//! The closed sets of values a tray item carries: status, category, and the actions a script
//! can trigger on it.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) enum SystrayStatus {
    Active,
    Passive,
    NeedsAttention,
    Unknown,
}

/// An action a script asks a tray item to perform, (de)serialised as its bare name.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(try_from = "String", into = "String")]
pub enum SystrayActionName {
    /// Left click: `ContextMenu` if the item sets `ItemIsMenu` (no fallback), else `Activate`
    /// falling back to `SecondaryActivate`.
    Primary,
    /// Calls the item's `ContextMenu`, falling back to `SecondaryActivate` and then `Activate`.
    ContextMenu,
    /// Calls the item's `Activate` method at the pointer position, or (0, 0) without one.
    Activate,
    /// Calls the item's `SecondaryActivate` method (a middle click in the SNI spec).
    SecondaryActivate,
    /// Sends the item a vertical `Scroll` of -1.
    ScrollUp,
    /// Sends the item a vertical `Scroll` of +1.
    ScrollDown,
    /// Sends the item a horizontal `Scroll` of -1.
    ScrollLeft,
    /// Sends the item a horizontal `Scroll` of +1.
    ScrollRight,
    /// Any other name; kept so it round-trips, but the adapter only logs it as unrecognised.
    Other(String),
}

impl SystrayActionName {
    #[must_use]
    pub(crate) fn parse_str(name: &str) -> Self {
        match name {
            "Primary" => Self::Primary,
            "ContextMenu" => Self::ContextMenu,
            "Activate" => Self::Activate,
            "SecondaryActivate" => Self::SecondaryActivate,
            "ScrollUp" => Self::ScrollUp,
            "ScrollDown" => Self::ScrollDown,
            "ScrollLeft" => Self::ScrollLeft,
            "ScrollRight" => Self::ScrollRight,
            other => Self::Other(other.to_string()),
        }
    }

    #[must_use]
    pub(crate) const fn as_str(&self) -> &str {
        match self {
            Self::Primary => "Primary",
            Self::ContextMenu => "ContextMenu",
            Self::Activate => "Activate",
            Self::SecondaryActivate => "SecondaryActivate",
            Self::ScrollUp => "ScrollUp",
            Self::ScrollDown => "ScrollDown",
            Self::ScrollLeft => "ScrollLeft",
            Self::ScrollRight => "ScrollRight",
            Self::Other(other) => other.as_str(),
        }
    }
}

impl From<&str> for SystrayActionName {
    fn from(name: &str) -> Self {
        Self::parse_str(name)
    }
}

impl From<String> for SystrayActionName {
    fn from(name: String) -> Self {
        Self::parse_str(&name)
    }
}

impl From<SystrayActionName> for String {
    fn from(action: SystrayActionName) -> Self {
        action.as_str().to_string()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(try_from = "String", into = "String")]
pub(crate) enum SystrayCategory {
    ApplicationStatus,
    Communications,
    SystemServices,
    Hardware,
    Other(String),
}

impl SystrayCategory {
    #[must_use]
    pub(crate) fn parse_str(category: &str) -> Self {
        match category {
            "ApplicationStatus" => Self::ApplicationStatus,
            "Communications" => Self::Communications,
            "SystemServices" => Self::SystemServices,
            "Hardware" => Self::Hardware,
            other => Self::Other(other.to_string()),
        }
    }

    #[must_use]
    pub(crate) const fn as_str(&self) -> &str {
        match self {
            Self::ApplicationStatus => "ApplicationStatus",
            Self::Communications => "Communications",
            Self::SystemServices => "SystemServices",
            Self::Hardware => "Hardware",
            Self::Other(other) => other.as_str(),
        }
    }
}

impl From<&str> for SystrayCategory {
    fn from(category: &str) -> Self {
        Self::parse_str(category)
    }
}

impl From<String> for SystrayCategory {
    fn from(category: String) -> Self {
        Self::parse_str(&category)
    }
}

impl From<SystrayCategory> for String {
    fn from(cat: SystrayCategory) -> Self {
        cat.as_str().to_string()
    }
}
