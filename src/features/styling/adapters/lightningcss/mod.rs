//! The lightningcss-backed CSS parser: turns stylesheet source into indexed rules and
//! re-parses single declarations after `var()` substitution.
pub mod adapter;
pub mod parsed_stylesheet;
pub mod properties;
pub mod reparser;
pub mod selector;
#[cfg(test)]
mod tests;

pub use adapter::*;
pub use reparser::*;
