//! The signal hub, the signal kinds modules subscribe to, and the state they carry.

pub mod hub;
pub mod hyprland;
pub mod kind;
pub mod monitor_info;
pub mod streams;
#[cfg(test)]
pub mod tests;

pub use hub::*;
pub(crate) use hyprland::*;
pub use kind::*;
