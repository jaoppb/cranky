//! Whether one input of a module's render changed since the last one.

/// Whether something a render depends on changed since the last render, which decides whether
/// the module is laid out or redrawn again.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Change {
    /// It changed: the module must react.
    Changed,
    /// It is as it was.
    Unchanged,
}

impl Change {
    /// Whether this is [`Change::Changed`].
    #[must_use]
    pub(crate) const fn is_changed(self) -> bool {
        matches!(self, Self::Changed)
    }
}

impl From<bool> for Change {
    /// `true` is [`Change::Changed`].
    fn from(changed: bool) -> Self {
        if changed {
            Self::Changed
        } else {
            Self::Unchanged
        }
    }
}
