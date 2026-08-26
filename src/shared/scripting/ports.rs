use crate::app::registry::ModuleError;
use crate::features::module_runtime::ports::AnyModulePort;
use crate::shared::config::domain::{EngineId, FileExtension};

pub trait ScriptEnginePort: Send + Sync {
    #[must_use]
    fn id(&self) -> EngineId;
    #[must_use]
    fn file_extension(&self) -> FileExtension;
    /// # Errors
    ///
    /// Returns `ModuleError` if module loading fails.
    fn load_module(&self, name: &str, source: &str) -> Result<Box<dyn AnyModulePort>, ModuleError>;
}
