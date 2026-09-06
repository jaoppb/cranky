use thiserror::Error;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum ColorError {
    #[error("Empty color string")]
    Empty,
    #[error("No colors found in input '{0}'")]
    NoColors(String),
    #[error("Invalid color format: '{0}'")]
    InvalidFormat(String),
    #[error("Invalid angle value: '{0}'")]
    InvalidAngle(String),
}
