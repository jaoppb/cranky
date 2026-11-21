pub mod dto;
mod event_parser;
pub mod hyprland_adapter;
pub mod hyprland_provider;
pub mod inconsistency;
mod signal_loop;
#[cfg(test)]
mod tests;

pub(crate) use hyprland_adapter::*;

pub mod hyprland {
    pub(crate) use super::*;
}
