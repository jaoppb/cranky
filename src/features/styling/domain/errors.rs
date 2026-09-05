use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum StylingError {
    #[error(
        "Invalid stylesheet name '{0}': must be non-empty alphanumeric with '-' or '_', without path separators or extensions"
    )]
    InvalidStyleSheetName(String),
    #[error("Invalid class name '{0}': must be a valid CSS identifier")]
    InvalidClassName(String),
    #[error("Invalid element ID '{0}': must be a valid CSS identifier")]
    InvalidElementId(String),
    #[error("Invalid progress value {0}: must be within range [0.0, 1.0]")]
    InvalidProgressValue(String),
    #[error("Invalid opacity value {0}: must be within range [0.0, 1.0]")]
    InvalidOpacity(String),
    #[error("Invalid flex value: {0}")]
    InvalidFlexValue(String),
    #[error("Invalid CSS length: {0}")]
    InvalidLength(String),
    #[error("CSS parser error: {0}")]
    ParserError(String),
    #[error("Style loader error: {0}")]
    LoaderError(String),
}
