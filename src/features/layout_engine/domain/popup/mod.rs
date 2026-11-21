pub mod chain;
pub mod styled;
pub mod target;
#[cfg(test)]
mod tests;

#[cfg(test)]
pub(crate) use chain::PopupCycle;
pub(crate) use chain::{ancestors, descendants_deepest_first};
pub(crate) use styled::{ActivePanel, AnchoredPopup, StyledPanel, StyledPopup};
pub(crate) use target::{FloatingKind, PopupExclusivity, PopupTarget};
