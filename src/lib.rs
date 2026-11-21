//! Cranky, a modular Wayland bar for Hyprland.
//!
//! Every visual element is a Lua or Rhai script run as its own actor: a
//! module renders a virtual DOM, which is diffed, styled from CSS, laid out
//! with taffy and painted into a shared-memory buffer on its own Wayland
//! surface or subsurface. The crate follows feature-sliced design with ports and adapters:
//! [`app`] composes everything, [`features`] holds the vertical slices and
//! [`shared`] the cross-cutting infrastructure. The binary only calls
//! [`app::run`].

pub mod app;
pub mod features;
pub mod shared;
pub mod utils;

#[cfg(test)]
#[macro_use]
pub mod test_utils;
