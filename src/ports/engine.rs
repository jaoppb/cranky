use crate::domain::config::{EngineId, FileExtension};
use crate::modules::ModuleError;
use crate::ports::registry::AnyModulePort;

pub trait ScriptEnginePort: Send + Sync {
    fn id(&self) -> EngineId;
    fn file_extension(&self) -> FileExtension;
    fn load_module(&self, name: &str, source: &str) -> Result<Box<dyn AnyModulePort>, ModuleError>;
}
