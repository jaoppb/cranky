#![allow(
    clippy::as_conversions,
    clippy::arithmetic_side_effects,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::ignored_unit_patterns,
    clippy::pedantic,
    clippy::nursery,
    clippy::expect_used,
    clippy::indexing_slicing
)]

pub mod adapter;
pub mod command;
pub mod dispatch_output;
pub mod dispatch_registry;
pub mod dispatch_seat;
pub mod dispatch_xdg;
pub mod display_server;
pub mod floating_anchor;
pub mod floating_render;
pub mod floating_surface;
pub mod render;
pub mod state;
pub mod surface_handler;
pub mod surface_manager;
pub mod types;

#[cfg(test)]
mod tests;

pub use adapter::WaylandAdapter;
pub use command::SurfaceCommand;
pub use surface_manager::WaylandSurfaceManager;
