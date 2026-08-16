use super::domain::AppEnvironment;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum EnvironmentError {
    #[error("Missing required environment variable: {0}")]
    MissingVariable(String),
}

pub trait EnvironmentPort: Send + Sync {
    fn read_environment(&self) -> Result<AppEnvironment, EnvironmentError>;
}
