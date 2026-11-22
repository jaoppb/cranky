//! The per-module, per-monitor render pipeline: diff the script's new `VNode` tree, lay it out
//! (with any open popup, panel or tooltip), and paint it into a buffer.
pub mod context;
pub mod diff_phase;
mod embed_state;
pub mod floating_phase;
pub mod layout_phase;
pub mod measurer;
pub mod outcome;
pub mod paint_phase;
pub mod pipeline;
pub mod services;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_changes;
#[cfg(test)]
mod tests_inputs;
#[cfg(test)]
mod tests_layout;

pub use context::*;
pub use floating_phase::*;
pub use outcome::*;
pub(crate) use paint_phase::*;
pub use pipeline::*;
pub(crate) use services::RenderServices;
