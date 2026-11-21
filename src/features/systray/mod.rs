//! System tray: hosts `StatusNotifierItem` icons over the D-Bus session bus and publishes them
//! to modules through the hub's systray signal.
pub mod adapters;
pub mod domain;
pub mod ports;
