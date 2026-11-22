//! The Rhai engine: compiles a module script and runs its `init`, `metadata`, `refresh` and
//! `render` functions.
pub mod api;
mod dsl_actions;
mod dsl_elements;
mod dsl_leaf;
mod dsl_sys;
mod helpers;
mod metadata;
mod module;
mod module_impl;
mod module_port;
mod script;
mod sync;
#[cfg(test)]
mod tests;

pub use module::RhaiModule;
