use thiserror::Error;

#[derive(Debug, Error)]
pub enum LayoutError {
    #[error("Failed to compute layout: {0}")]
    EngineError(String),
}
