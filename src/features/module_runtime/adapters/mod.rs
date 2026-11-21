//! The Tokio-side adapter that drives a module actor: its event loop.
pub mod event_loop;

pub(super) use event_loop::EventLoop;
