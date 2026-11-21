//! The errors raised when styling input (names, values, CSS source, stylesheet files) is
//! invalid.
use thiserror::Error;

/// Why a styling value, stylesheet name or stylesheet could not be accepted; each variant
/// carries the offending input or the underlying error text.
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum StylingError {
    /// A stylesheet name that is empty, has characters other than alphanumerics, `-` and `_`, or
    /// carries a path or extension.
    #[error(
        "Invalid stylesheet name '{0}': must be non-empty alphanumeric with '-' or '_', without path separators or extensions"
    )]
    InvalidStyleSheetName(String),
    /// A class name that is not a valid CSS identifier.
    #[error("Invalid class name '{0}': must be a valid CSS identifier")]
    InvalidClassName(String),
    /// An element id that is not a valid CSS identifier.
    #[error("Invalid element ID '{0}': must be a valid CSS identifier")]
    InvalidElementId(String),
    /// A progress value outside `0.0..=1.0`.
    #[error("Invalid progress value {0}: must be within range [0.0, 1.0]")]
    InvalidProgressValue(String),
    /// An opacity outside `0.0..=1.0`.
    #[error("Invalid opacity value {0}: must be within range [0.0, 1.0]")]
    InvalidOpacity(String),
    /// A `flex-grow` or `flex-shrink` factor that is NaN or negative.
    #[error("Invalid flex value: {0}")]
    InvalidFlexValue(String),
    /// A pixel length or percentage that is NaN or negative.
    #[error("Invalid CSS length: {0}")]
    InvalidLength(String),
    /// lightningcss rejected the stylesheet source.
    #[error("CSS parser error: {0}")]
    ParserError(String),
    /// A stylesheet was found in no search path, or its directory or watcher could not be created.
    #[error("Style loader error: {0}")]
    LoaderError(String),
}
