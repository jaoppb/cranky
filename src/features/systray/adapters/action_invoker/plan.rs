//! Which calls an action name stands for.
use super::ItemCall;
use crate::shared::primitives::geometry::Position;

/// What an action name asks of the item.
pub(super) enum ActionPlan<'name> {
    /// A primary click, which depends on `ItemIsMenu`.
    Primary,
    /// `ContextMenu` with its two fallbacks.
    ContextMenu,
    /// One call with no fallback.
    Single(ItemCall),
    /// A name the tray does not know.
    Unrecognized(&'name str),
}

/// The plan for the action `name`, at `position`.
pub(super) fn plan(name: &str, position: Position) -> ActionPlan<'_> {
    match name {
        "Primary" => ActionPlan::Primary,
        "Activate" => ActionPlan::Single(ItemCall::Activate(position)),
        "SecondaryActivate" => ActionPlan::Single(ItemCall::SecondaryActivate(position)),
        "ContextMenu" => ActionPlan::ContextMenu,
        "ScrollUp" => ActionPlan::Single(ItemCall::Scroll(-1, "vertical")),
        "ScrollDown" => ActionPlan::Single(ItemCall::Scroll(1, "vertical")),
        "ScrollLeft" => ActionPlan::Single(ItemCall::Scroll(-1, "horizontal")),
        "ScrollRight" => ActionPlan::Single(ItemCall::Scroll(1, "horizontal")),
        other => ActionPlan::Unrecognized(other),
    }
}
