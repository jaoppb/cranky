use crate::shared::config::domain::{EngineId, FileExtension};
pub mod lua;
pub mod rhai;

use crate::app::registry::ModuleError;
use crate::features::module_runtime::ports::AnyModulePort;
use crate::shared::scripting::ports::ScriptEnginePort;

pub struct RhaiEngineAdapter;

impl ScriptEnginePort for RhaiEngineAdapter {
    fn id(&self) -> EngineId {
        EngineId::new("rhai")
    }

    fn file_extension(&self) -> FileExtension {
        FileExtension::new("rhai")
    }

    fn load_module(&self, name: &str, source: &str) -> Result<Box<dyn AnyModulePort>, ModuleError> {
        let m = rhai::RhaiModule::new(name.to_string(), source)?;
        Ok(Box::new(m))
    }
}

pub struct LuaEngineAdapter;

impl ScriptEnginePort for LuaEngineAdapter {
    fn id(&self) -> EngineId {
        EngineId::new("lua")
    }

    fn file_extension(&self) -> FileExtension {
        FileExtension::new("lua")
    }

    fn load_module(&self, name: &str, source: &str) -> Result<Box<dyn AnyModulePort>, ModuleError> {
        let m = lua::LuaModule::new(name.to_string(), source.to_string());
        Ok(Box::new(m))
    }
}
