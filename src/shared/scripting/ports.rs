//! The script engine port the builtins loader loads module sources through, and its error.
use crate::features::module_runtime::ports::AnyModulePort;
use crate::shared::config::domain::{EngineId, FileExtension};
use thiserror::Error;

/// Failure to compile or set up a module script.
#[derive(Error, Debug, PartialEq, Eq)]
pub enum ModuleError {
    /// The engine rejected the script; `message` names the module and the engine's error.
    #[error("Internal module error: {message}")]
    Internal {
        /// The module name and the engine's error, as text.
        message: String,
    },
}

pub(crate) trait ScriptEnginePort: Send + Sync {
    #[must_use]
    fn id(&self) -> EngineId;
    #[must_use]
    fn file_extension(&self) -> FileExtension;
    /// # Errors
    ///
    /// Returns `ModuleError` if module loading fails.
    fn load_module(&self, name: &str, source: &str) -> Result<Box<dyn AnyModulePort>, ModuleError>;
}
