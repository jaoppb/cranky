//! The per-module actor task: waits on signal, layout and pointer events, re-renders the module
//! on each monitor that needs it, and dispatches the results to the display and layout channels.
mod dispatch;
pub mod events;
mod floating_dispatch;
mod outcome_dispatch;
mod pointer;
mod poll;
mod renderer;
mod runner;
#[cfg(test)]
mod tests;

pub(in crate::features::module_runtime) use runner::*;
