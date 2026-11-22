//! Specs for the floating content a node can open: popups, panels and the layer-shell options
//! panels carry.
pub mod anchor;
pub mod edge;
pub mod panel;
mod popup;
#[cfg(test)]
mod tests;
pub mod types;

pub(crate) use anchor::PanelAnchor;
pub(crate) use edge::{HorizontalAnchor, VerticalAnchor};
pub(crate) use panel::{PanelSpec, PanelSurface};
pub use popup::{AnchorDirection, PopupOffset, PopupSpec};
pub(crate) use types::{ExclusiveZone, KeyboardInteractivity, PanelLayer};
