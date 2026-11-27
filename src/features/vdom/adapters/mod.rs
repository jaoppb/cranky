//! The tree differ that compares a module's previous and new `VNode` trees into a patch.
pub mod children;
pub mod differ;
mod floating_differ;
pub mod kind_differ;

pub use differ::*;

#[cfg(test)]
mod tests;
