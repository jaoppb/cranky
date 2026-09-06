use thiserror::Error;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum ModuleInitError {
    #[error("Script evaluation error: {0}")]
    ScriptError(String),
    #[error("Configuration error: {0}")]
    ConfigError(String),
    #[error("Internal module error: {0}")]
    Internal(String),
}

#[derive(Error, Debug, PartialEq, Eq)]
pub enum RegistryLoadError {
    #[error("Failed to initialize module '{module_name}': {source}")]
    ModuleInit {
        module_name: crate::shared::primitives::ModuleName,
        #[source]
        source: ModuleInitError,
    },
    #[error("Module not found: {0}")]
    ModuleNotFound(crate::shared::primitives::ModuleName),
    #[error("Unsupported engine '{engine}' for module '{module_name}'")]
    UnsupportedEngine {
        engine: String,
        module_name: crate::shared::primitives::ModuleName,
    },
    #[error("Internal registry error: {0}")]
    Internal(String),
}
