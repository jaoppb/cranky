//! The composition root: builds every adapter and module from the configuration and runs
//! the application loop that routes their commands.

mod adapter_supervisor;
mod bootstrap;
mod builtins;
mod commands;
mod registry;
pub mod state;
mod time_adapter;

pub use bootstrap::{BootError, run};

#[cfg(test)]
mod test_fakes;
