//! The Hyprland side of workspaces: queries its IPC socket, follows its event socket, and
//! keeps the hub's hyprland state current.
pub mod dto;
mod event_parser;
pub mod hyprland_adapter;
pub mod hyprland_provider;
pub mod inconsistency;
mod signal_loop;
#[cfg(test)]
mod tests;

pub(crate) use hyprland_adapter::*;

/// The adapter's public face under the name the app wires it by
/// (`adapters::hyprland::HyprlandAdapter`).
pub mod hyprland {
    pub(crate) use super::*;
}
