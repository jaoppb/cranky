use crate::app::registry::ModuleError;
use crate::features::module_runtime::ports::AnyModulePort;
use crate::shared::config::domain::{EngineId, FileExtension};

pub trait ScriptEnginePort: Send + Sync {
    fn id(&self) -> EngineId;
    fn file_extension(&self) -> FileExtension;
    fn load_module(&self, name: &str, source: &str) -> Result<Box<dyn AnyModulePort>, ModuleError>;
}
