//! The only `unsafe` code Cranky needs, behind safe functions and types.
//!
//! The main crate forbids `unsafe_code`. Two things it needs cannot be written
//! without it, so they live here, where the lint is not enabled for the whole
//! crate: mapping the shared-memory file a Wayland buffer pool is backed by, and
//! moving a taffy tree between the threads of the async runtime.

mod mapping;
mod send_tree;

pub use mapping::SharedMapping;
pub use send_tree::SendTaffyTree;
