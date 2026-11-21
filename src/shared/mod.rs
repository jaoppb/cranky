//! Infrastructure every feature relies on: configuration, `DBus`, the environment, the signal
//! hub, rendering, scripting and Wayland.

pub mod config;
pub mod dbus;
pub mod env;
pub mod events;
pub mod rendering;
pub mod scripting;
pub mod wayland;

pub mod primitives;
