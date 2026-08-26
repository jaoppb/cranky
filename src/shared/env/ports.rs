use super::domain::AppEnvironment;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum EnvironmentError {
    #[error("Missing required environment variable: {0}")]
    MissingVariable(String),
}

pub trait EnvironmentPort: Send + Sync {
    /// Reads the current application environment.
    ///
    /// # Errors
    ///
    /// Returns [`EnvironmentError::MissingVariable`] if a required environment variable is not set.
    fn read_environment(&self) -> Result<AppEnvironment, EnvironmentError>;
}
