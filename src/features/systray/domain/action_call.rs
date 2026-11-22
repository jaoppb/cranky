//! An action a tray item is asked to perform, with the pointer position it was triggered at.
use super::enums::SystrayActionName;
use crate::shared::primitives::geometry::Position;

/// One call of an SNI action on a tray item: which action, and the pointer position its D-Bus
/// method receives, or none when the trigger had no position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystrayActionCall {
    name: SystrayActionName,
    position: Option<Position>,
}

impl SystrayActionCall {
    /// `name` triggered at `position`, when the trigger had one.
    #[must_use]
    pub const fn new(name: SystrayActionName, position: Option<Position>) -> Self {
        Self { name, position }
    }

    #[must_use]
    pub(crate) const fn name(&self) -> &SystrayActionName {
        &self.name
    }

    /// The pointer position the action's D-Bus method receives; `None` is sent as (0, 0).
    #[must_use]
    pub(crate) const fn position(&self) -> Option<Position> {
        self.position
    }
}
