//! Errors raised while initializing a module or loading it into the registry.
use thiserror::Error;

/// Why a script module failed to initialize or to run one of its functions.
#[derive(Error, Debug, PartialEq, Eq)]
pub enum ModuleInitError {
    /// The script engine failed to evaluate or call the script.
    #[error("Script evaluation error: {0}")]
    ScriptError(String),
    /// The module's configuration could not be converted for the script.
    #[error("Configuration error: {0}")]
    ConfigError(String),
    /// Reserved for adapter failures that are neither script nor config errors; no adapter raises
    /// it today.
    #[error("Internal module error: {0}")]
    Internal(String),
}

/// Why the registry could not load or reload a module.
#[derive(Error, Debug, PartialEq, Eq)]
pub enum RegistryLoadError {
    /// The module's script was found but its `init` failed.
    #[error("Failed to initialize module '{module_name}': {source}")]
    ModuleInit {
        /// The module that failed to initialize.
        module_name: crate::shared::primitives::ModuleName,
        /// What went wrong inside `init`.
        #[source]
        source: ModuleInitError,
    },
    /// No script for this name (and pinned engine) could be read and loaded from the user or
    /// built-in modules directory.
    #[error("Module not found: {0}")]
    ModuleNotFound(crate::shared::primitives::ModuleName),
    /// The config pins an engine name that is not `lua` or `rhai`.
    #[error("Unsupported engine '{engine}' for module '{module_name}'")]
    UnsupportedEngine {
        /// The engine name the config asked for.
        engine: String,
        /// The module it was asked for.
        module_name: crate::shared::primitives::ModuleName,
    },
    /// Creating the shared modules directory for the built-in scripts failed with an I/O error.
    #[error("Internal registry error: {0}")]
    Internal(String),
}
