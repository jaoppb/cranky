//! The emulated implicit grab: which buttons are held, and where the target
//! they froze sits on the pointer surface.
use crate::shared::primitives::geometry::Position;
use std::collections::BTreeSet;

/// The held buttons and, while any is, the frozen target's origin on the
/// pointer surface: the child layer's origin at press time, or (0, 0) for a
/// module on its own surface.
#[derive(Debug)]
pub(crate) struct PointerGrab {
    held: BTreeSet<u32>,
    origin: Position,
}

impl Default for PointerGrab {
    fn default() -> Self {
        Self {
            held: BTreeSet::new(),
            origin: Position::new(0, 0),
        }
    }
}

impl PointerGrab {
    pub(super) fn is_active(&self) -> bool {
        !self.held.is_empty()
    }

    /// Starts a grab frozen at `origin`; a grab already active keeps its own.
    pub(super) fn begin(&mut self, origin: Position) {
        if self.held.is_empty() {
            self.origin = origin;
        }
    }

    pub(super) fn hold(&mut self, button: u32) {
        self.held.insert(button);
    }

    /// Returns whether this release ended the grab; releasing a button that
    /// is not held changes nothing.
    pub(super) fn release(&mut self, button: u32) -> bool {
        self.held.remove(&button) && self.held.is_empty()
    }

    pub(super) fn cancel(&mut self) {
        self.held.clear();
    }

    /// `pos` relative to the frozen target's origin; may be negative or past
    /// the target's size, which the target hit-tests itself.
    pub(super) const fn local(&self, pos: Position) -> Position {
        Position::new(
            pos.x().saturating_sub(self.origin.x()),
            pos.y().saturating_sub(self.origin.y()),
        )
    }
}
