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
#[cfg(test)]
mod tests;

pub use context::*;
pub use floating_phase::*;
pub use outcome::*;
pub(crate) use paint_phase::*;
pub use pipeline::*;
