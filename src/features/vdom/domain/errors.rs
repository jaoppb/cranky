//! Errors raised while building VDOM values.
use thiserror::Error;

/// Why a VDOM value could not be built.
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum VdomError {
    /// A node key was empty or only whitespace.
    #[error("Invalid NodeKey: {0}")]
    InvalidNodeKey(String),
}
