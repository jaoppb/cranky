//! The Wayland side of the bar: the connection, layer-shell and popup surfaces, shared-memory
//! buffers and input.
pub mod adapters;
pub mod composite;
pub(crate) mod domain;
pub mod ports;
mod surface_parent;
