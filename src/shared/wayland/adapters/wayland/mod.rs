//! The Wayland client: connection, globals, layer-shell bars, popups, input dispatch and buffer
//! submission.
mod adapter;
pub mod command;
mod dispatch_keyboard;
mod dispatch_objects;
mod dispatch_output;
mod dispatch_pointer;
pub mod dispatch_pointer_button;
mod dispatch_registry;
mod dispatch_seat;
mod dispatch_xdg;
mod display_server;
mod floating_anchor;
mod floating_bounds;
mod floating_composite;
mod floating_render;
mod floating_setup;
mod floating_surface;
mod floating_teardown;
mod floating_update;
mod input_region;
pub mod keyboard;
mod render;
pub mod state;
mod surface_handler;
mod surface_manager;
mod tooltip_anchor;
pub mod types;

#[cfg(test)]
pub mod tests;

pub use adapter::WaylandAdapter;
pub use surface_manager::WaylandSurfaceManager;
