//! Whether a monitor holds the compositor's focus.
use serde::{Deserialize, Serialize};

/// Whether a monitor is the one the compositor currently focuses, which decides its focused or
/// unfocused styling. Serialized as a plain `bool` (`true` when focused), as the scripts see it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "bool", into = "bool")]
pub enum MonitorFocus {
    /// The compositor focuses this monitor.
    Focused,
    /// Another monitor, or none, holds the focus.
    Unfocused,
}

impl MonitorFocus {
    /// Whether this is [`MonitorFocus::Focused`].
    #[must_use]
    pub(crate) const fn is_focused(self) -> bool {
        matches!(self, Self::Focused)
    }
}

impl From<bool> for MonitorFocus {
    fn from(is_focused: bool) -> Self {
        if is_focused {
            Self::Focused
        } else {
            Self::Unfocused
        }
    }
}

impl From<MonitorFocus> for bool {
    fn from(focus: MonitorFocus) -> Self {
        focus.is_focused()
    }
}
