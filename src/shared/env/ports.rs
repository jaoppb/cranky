//! The port the app reads its environment through, and the error it fails with.
use super::domain::AppEnvironment;

/// Why the environment could not be read.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum EnvironmentError {
    /// A required variable (`HOME` or `XDG_RUNTIME_DIR`) is unset; carries its name.
    #[error("Missing required environment variable: {0}")]
    MissingVariable(String),
}

pub(crate) trait EnvironmentPort: Send + Sync {
    /// Reads the current application environment.
    ///
    /// # Errors
    ///
    /// Returns [`EnvironmentError::MissingVariable`] if a required environment variable is not set.
    fn read_environment(&self) -> Result<AppEnvironment, EnvironmentError>;
}
