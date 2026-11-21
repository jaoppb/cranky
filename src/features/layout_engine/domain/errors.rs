//! Errors a layout pass can return.
use thiserror::Error;

/// Why a layout pass failed; the taffy adapter drops its cached tree on any error.
#[derive(Debug, Error)]
pub enum LayoutError {
    /// Taffy rejected an operation; carries its error message.
    #[error("Failed to compute layout: {0}")]
    EngineError(String),
}
