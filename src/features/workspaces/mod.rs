//! Hyprland workspaces and monitors: reads the compositor's state over its IPC sockets and
//! publishes every change to modules through the hub's hyprland signal.
pub mod adapters;
pub mod domain;
mod ports;
