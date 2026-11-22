//! Serde DTOs mirroring `config.toml`, each converting into its domain config type.
pub mod config;
mod helpers;
mod margin;
mod module;
mod root;
mod style;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_config;
#[cfg(test)]
mod tests_defaults;

pub(crate) use config::*;
#[cfg(test)]
use margin::*;
#[cfg(test)]
use style::*;
