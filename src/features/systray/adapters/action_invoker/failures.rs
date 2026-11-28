//! The errors of a `ContextMenu` and of its two fallbacks.
use std::fmt;

/// The errors of a `ContextMenu` and of its `SecondaryActivate` fallback, before the last
/// fallback, `Activate`, is tried.
pub(super) struct EarlierFailures {
    /// What `ContextMenu` failed with.
    context_menu: zbus::Error,
    /// What the `SecondaryActivate` fallback failed with.
    secondary: zbus::Error,
}

impl EarlierFailures {
    /// The failures of `context_menu` and of its `secondary` fallback.
    pub(super) const fn new(context_menu: zbus::Error, secondary: zbus::Error) -> Self {
        Self {
            context_menu,
            secondary,
        }
    }

    /// Every failure of the chain, once `Activate` failed with `activate` too.
    pub(super) fn and_activate(self, activate: zbus::Error) -> ContextMenuFailures {
        let Self {
            context_menu,
            secondary,
        } = self;
        ContextMenuFailures {
            context_menu,
            secondary,
            activate,
        }
    }
}

/// The three errors of a `ContextMenu` whose two fallbacks failed too.
pub(super) struct ContextMenuFailures {
    /// What `ContextMenu` failed with.
    context_menu: zbus::Error,
    /// What the `SecondaryActivate` fallback failed with.
    secondary: zbus::Error,
    /// What the last `Activate` fallback failed with.
    activate: zbus::Error,
}

impl fmt::Display for ContextMenuFailures {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            context_menu,
            secondary,
            activate,
        } = self;
        write!(
            formatter,
            "{context_menu} (fallback SecondaryActivate: {secondary}, Activate: {activate})"
        )
    }
}
