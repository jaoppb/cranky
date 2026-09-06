pub mod dto;
pub mod event_parser;
pub mod hyprland_adapter;
pub mod hyprland_provider;
pub mod inconsistency;
pub mod signal_loop;
#[cfg(test)]
mod tests;

pub use dto::*;
pub use event_parser::*;
pub use hyprland_adapter::*;
pub use hyprland_provider::*;
pub use inconsistency::*;
pub use signal_loop::*;

pub mod hyprland {
    pub use super::*;
}
