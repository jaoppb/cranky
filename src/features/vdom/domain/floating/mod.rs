//! Specs for the floating content a node can open: popups, panels and the layer-shell options
//! panels carry.
pub mod anchor;
pub mod panel;
mod popup;
#[cfg(test)]
mod tests;
pub mod types;

pub(crate) use anchor::{HorizontalAnchor, PanelAnchor, VerticalAnchor};
pub(crate) use panel::PanelSpec;
pub use popup::{AnchorDirection, PopupOffset, PopupSpec};
pub(crate) use types::{ExclusiveZone, KeyboardInteractivity, PanelLayer};
