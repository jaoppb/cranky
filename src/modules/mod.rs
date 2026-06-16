use crate::domain::config::ModuleConfig;
use crate::ports::canvas::Canvas;
use crate::ports::registry::{AnyModulePort, ModuleRegistryPort};
use crate::domain::signals::{SignalHub, SignalKind};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ModuleError {
    #[error("Module '{module_name}' not found")]
    ModuleNotFound { module_name: String },
    #[error("Script error in module '{module_name}': {message}")]
    ScriptError { module_name: String, message: String },
    #[error("Internal module error: {message}")]
    Internal { message: String },
}

use crate::domain::{ModuleId, MonitorId, geometry::Size};
use std::collections::HashMap;

pub mod lua;
pub mod rhai;
pub mod actor;

pub struct ModuleRegistry {
    modules: HashMap<ModuleId, Box<dyn AnyModulePort>>,
    left_modules: Vec<ModuleId>,
    center_modules: Vec<ModuleId>,
    right_modules: Vec<ModuleId>,
}

impl ModuleRegistry {
    pub fn new() -> Self {
        Self {
            modules: HashMap::new(),
            left_modules: Vec::new(),
            center_modules: Vec::new(),
            right_modules: Vec::new(),
        }
    }

    fn load_section(
        &mut self,
        configs: &[ModuleConfig],
        bar_config: &crate::domain::config::BarConfig,
        next_id: &mut u32,
    ) -> Result<Vec<ModuleId>, String> {
        let mut ids = Vec::new();
        for config in configs {
            if !config.is_enabled() {
                continue;
            }

            let id = ModuleId::new(*next_id);
            *next_id += 1;

            let mut module: Box<dyn AnyModulePort> = match config.name() {
                "hour" => Box::new(lua::LuaModule::built_in("hour")
                    .ok_or_else(|| "Module hour not found".to_string())?),
                "workspace" => Box::new(lua::LuaModule::built_in("workspace")
                    .ok_or_else(|| "Module workspace not found".to_string())?),
                "applet" => Box::new(lua::LuaModule::built_in("applet")
                    .ok_or_else(|| "Module applet not found".to_string())?),
                "metrics" => Box::new(lua::LuaModule::built_in("metrics")
                    .ok_or_else(|| "Module metrics not found".to_string())?),
                name => {
                    // Try to load as lua first, then rhai
                    if let Some(m) = lua::LuaModule::external(name) {
                        Box::new(m)
                    } else if let Some(m) = rhai::RhaiModule::external(name) {
                        Box::new(m)
                    } else {
                        return Err(ModuleError::ModuleNotFound { module_name: name.to_string() }.to_string());
                    }
                }
            };

            module.init(config, bar_config).map_err(|e| e.to_string())?;
            self.modules.insert(id, module);
            ids.push(id);
        }
        Ok(ids)
    }
}

#[async_trait::async_trait]
impl ModuleRegistryPort for ModuleRegistry {
    fn left_modules(&self) -> Vec<ModuleId> {
        self.left_modules.clone()
    }

    fn center_modules(&self) -> Vec<ModuleId> {
        self.center_modules.clone()
    }

    fn right_modules(&self) -> Vec<ModuleId> {
        self.right_modules.clone()
    }

    fn load(&mut self, config: &crate::domain::config::Config) -> Result<(), String> {
        self.modules.clear();
        let mut next_id = 0;
        
        self.left_modules = self.load_section(config.modules().left(), config.bar(), &mut next_id).map_err(|e| e.to_string())?;
        self.center_modules = self.load_section(config.modules().center(), config.bar(), &mut next_id).map_err(|e| e.to_string())?;
        self.right_modules = self.load_section(config.modules().right(), config.bar(), &mut next_id).map_err(|e| e.to_string())?;
        
        Ok(())
    }

    fn spawn_all(
        &mut self,
        hub: std::sync::Arc<SignalHub>,
        surface_manager: crate::ports::surface::DynSurfaceManager,
        command_tx: tokio::sync::mpsc::Sender<crate::domain::commands::AppCommand>
    ) -> std::collections::HashMap<ModuleId, tokio::sync::watch::Sender<std::collections::HashMap<MonitorId, crate::domain::geometry::Rect>>> {
        let mut layout_senders = std::collections::HashMap::new();

        for (id, module) in self.modules.drain().collect::<Vec<_>>() {
            let (layout_tx, layout_rx) = tokio::sync::watch::channel(std::collections::HashMap::new());
            layout_senders.insert(id, layout_tx);

            let ctx = crate::ports::registry::ModuleContext {
                id,
                hub: hub.clone(),
                surface_manager: surface_manager.clone(),
                command_tx: command_tx.clone(),
                layout_rx,
            };

            actor::ModuleActor::new(module, ctx).spawn();
        }

        layout_senders
    }

    fn clear(&mut self) {
        self.modules.clear();
        self.left_modules.clear();
        self.center_modules.clear();
        self.right_modules.clear();
    }

    async fn register_dbus_subscriptions(&self, dbus: &mut dyn crate::ports::DBusPort) {
        for module in self.modules.values() {
            for kind in module.subscriptions() {
                if let crate::domain::signals::SignalKind::DBus(sub) = kind {
                    if let Err(e) = dbus.subscribe(sub).await {
                        tracing::error!("Failed to subscribe to DBus: {}", e);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::config::dto::ConfigDto;
    use crate::ports::font::FontValidatorPort;

    struct MockValidator;
    impl FontValidatorPort for MockValidator {
        fn is_valid_family(&self, _family: &str) -> bool { true }
    }

    #[test]
    fn test_module_registry_load() {
        let mut registry = ModuleRegistry::new();
        let toml_str = r##"
            [bar]
            [modules]
            left = [{ name = "hour", enable = true }]
            center = []
            right = []
        "##;
        let dto: ConfigDto = toml::from_str(toml_str).unwrap();
        let config = dto.to_domain(&MockValidator);

        registry.load(&config).unwrap();
        assert_eq!(registry.left_modules().len(), 1);
    }

}
