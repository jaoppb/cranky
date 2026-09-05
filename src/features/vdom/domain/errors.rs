use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum VdomError {
    #[error("Invalid NodeKey: {0}")]
    InvalidNodeKey(String),
}
