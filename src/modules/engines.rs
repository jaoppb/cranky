use crate::domain::config::{EngineId, FileExtension};
use crate::modules::{lua, rhai, ModuleError};
use crate::ports::registry::AnyModulePort;
use crate::ports::ScriptEnginePort;

pub struct RhaiEngineAdapter;

impl ScriptEnginePort for RhaiEngineAdapter {
    fn id(&self) -> EngineId {
        EngineId::new("rhai")
    }

    fn file_extension(&self) -> FileExtension {
        FileExtension::new("rhai")
    }

    fn load_module(&self, name: &str, source: &str) -> Result<Box<dyn AnyModulePort>, ModuleError> {
        rhai::RhaiModule::new(name.to_string(), source)
            .map(|m| Box::new(m) as Box<dyn AnyModulePort>)
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
        Ok(Box::new(m) as Box<dyn AnyModulePort>)
    }
}
