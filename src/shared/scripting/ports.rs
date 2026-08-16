use crate::shared::config::domain::{EngineId, FileExtension};
use crate::app::registry::ModuleError;
use crate::features::module_runtime::ports::AnyModulePort;

pub trait ScriptEnginePort: Send + Sync {
    fn id(&self) -> EngineId;
    fn file_extension(&self) -> FileExtension;
    fn load_module(&self, name: &str, source: &str) -> Result<Box<dyn AnyModulePort>, ModuleError>;
}
