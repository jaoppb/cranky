//! Value types for Hyprland workspaces and monitors, free of IPC and JSON concerns.
pub mod identifiers;
pub mod monitor;
#[cfg(test)]
mod tests;
pub mod workspace;

pub(crate) use identifiers::*;
pub(crate) use monitor::*;
pub(crate) use workspace::*;
