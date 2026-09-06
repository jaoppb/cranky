use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SystrayStatus {
    Active,
    Passive,
    NeedsAttention,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(try_from = "String", into = "String")]
pub enum SystrayActionName {
    Primary,
    ContextMenu,
    Activate,
    SecondaryActivate,
    ScrollUp,
    ScrollDown,
    ScrollLeft,
    ScrollRight,
    Other(String),
}

impl SystrayActionName {
    #[must_use]
    pub fn parse_str(s: &str) -> Self {
        match s {
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
    pub const fn as_str(&self) -> &str {
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
    fn from(s: &str) -> Self {
        Self::parse_str(s)
    }
}

impl From<String> for SystrayActionName {
    fn from(s: String) -> Self {
        Self::parse_str(&s)
    }
}

impl From<SystrayActionName> for String {
    fn from(action: SystrayActionName) -> Self {
        action.as_str().to_string()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(try_from = "String", into = "String")]
pub enum SystrayCategory {
    ApplicationStatus,
    Communications,
    SystemServices,
    Hardware,
    Other(String),
}

impl SystrayCategory {
    #[must_use]
    pub fn parse_str(s: &str) -> Self {
        match s {
            "ApplicationStatus" => Self::ApplicationStatus,
            "Communications" => Self::Communications,
            "SystemServices" => Self::SystemServices,
            "Hardware" => Self::Hardware,
            other => Self::Other(other.to_string()),
        }
    }

    #[must_use]
    pub const fn as_str(&self) -> &str {
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
    fn from(s: &str) -> Self {
        Self::parse_str(s)
    }
}

impl From<String> for SystrayCategory {
    fn from(s: String) -> Self {
        Self::parse_str(&s)
    }
}

impl From<SystrayCategory> for String {
    fn from(cat: SystrayCategory) -> Self {
        cat.as_str().to_string()
    }
}
