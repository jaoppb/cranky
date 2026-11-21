//! Input and window-manager event types shared across features.

pub mod pointer;
#[cfg(test)]
mod tests;
pub mod window;

pub(crate) use pointer::{
    InteractionEvent, PointerButton, PointerEvent, PointerReceiver, PointerSender, PointerSerial,
    ScrollAxis, ScrollDelta, SurfaceKind, SurfaceLifecycleEvent,
};
pub(crate) use window::{WindowAddress, WindowManagerEvent, WindowTitle};
