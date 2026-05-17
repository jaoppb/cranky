use crate::config::ModuleConfig;
use crate::ports::canvas::Canvas;
use crate::domain::signals::SignalHub;
use crate::domain::errors::DomainError;
use crate::domain::{ModuleId, MonitorId, geometry::Size};
use std::collections::HashMap;

pub mod applet;
pub mod hour;
pub mod workspace;

/// A type-erased version of CrankyModule.
pub trait AnyModule: Send + Sync {
    fn init(
        &mut self,
        config: &ModuleConfig,
        bar_config: &crate::config::BarConfig,
    ) -> Result<(), DomainError>;

    fn attach(&mut self, hub: &SignalHub, target_id: ModuleId);

    fn refresh(&mut self, hub: &SignalHub);

    fn view(&self, canvas: &mut dyn Canvas, monitor: &MonitorId);

    fn measure(&self, canvas: &mut dyn Canvas, monitor: &MonitorId) -> Size;
}

pub trait CrankyModule: Send + Sync {
    type Config: for<'de> serde::Deserialize<'de> + Default + Send + Sync;

    fn init(
        &mut self,
        config: Self::Config,
        bar_config: &crate::config::BarConfig,
    ) -> Result<(), DomainError>;

    fn attach(&mut self, hub: &SignalHub, target_id: ModuleId);

    fn refresh(&mut self, hub: &SignalHub);

    fn view(&self, canvas: &mut dyn Canvas, monitor: &MonitorId);

    fn measure(&self, canvas: &mut dyn Canvas, monitor: &MonitorId) -> Size;
}

impl<T> AnyModule for T
where
    T: CrankyModule,
{
    fn init(
        &mut self,
        config: &ModuleConfig,
        bar_config: &crate::config::BarConfig,
    ) -> Result<(), DomainError> {
        let json_value = serde_json::to_value(config.options())
            .map_err(|e| DomainError::ConfigParseError { reason: e.to_string() })?;

        let typed_config: T::Config = serde_json::from_value(json_value)
            .map_err(|e| DomainError::ConfigParseError { reason: e.to_string() })?;

        self.init(typed_config, bar_config)
    }

    fn attach(&mut self, hub: &SignalHub, target_id: ModuleId) {
        self.attach(hub, target_id);
    }

    fn refresh(&mut self, hub: &SignalHub) {
        self.refresh(hub);
    }

    fn view(&self, canvas: &mut dyn Canvas, monitor: &MonitorId) {
        self.view(canvas, monitor);
    }

    fn measure(&self, canvas: &mut dyn Canvas, monitor: &MonitorId) -> Size {
        self.measure(canvas, monitor)
    }
}

pub struct ModuleRegistry {
    modules: HashMap<ModuleId, Box<dyn AnyModule>>,
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

    pub fn left_modules(&self) -> Vec<ModuleId> {
        self.left_modules.clone()
    }

    pub fn center_modules(&self) -> Vec<ModuleId> {
        self.center_modules.clone()
    }

    pub fn right_modules(&self) -> Vec<ModuleId> {
        self.right_modules.clone()
    }

    pub fn get(&self, id: ModuleId) -> Option<&dyn AnyModule> {
        self.modules.get(&id).map(|m| m.as_ref())
    }

    pub fn load(&mut self, config: &crate::config::Config) -> Result<(), DomainError> {
        self.modules.clear();
        let mut next_id = 0;
        
        self.left_modules = self.load_section(config.modules().left(), config.bar(), &mut next_id)?;
        self.center_modules = self.load_section(config.modules().center(), config.bar(), &mut next_id)?;
        self.right_modules = self.load_section(config.modules().right(), config.bar(), &mut next_id)?;
        
        Ok(())
    }

    fn load_section(
        &mut self,
        configs: &[ModuleConfig],
        bar_config: &crate::config::BarConfig,
        next_id: &mut u32,
    ) -> Result<Vec<ModuleId>, DomainError> {
        let mut ids = Vec::new();
        for config in configs {
            if !config.is_enabled() {
                continue;
            }

            let id = ModuleId::new(*next_id);
            *next_id += 1;

            let mut module: Box<dyn AnyModule> = match config.name() {
                "hour" => Box::new(hour::HourModule::new()),
                "applet" => Box::new(applet::AppletModule::new()),
                "workspace" => Box::new(workspace::WorkspaceModule::new()),
                name => return Err(DomainError::ModuleNotFound { module_name: name.to_string() }),
            };

            module.init(config, bar_config)?;
            self.modules.insert(id, module);
            ids.push(id);
        }
        Ok(ids)
    }

    pub fn attach_all(&mut self, hub: &SignalHub) {
        for (id, module) in self.modules.iter_mut() {
            module.attach(hub, *id);
        }
    }

    pub fn refresh_all(&mut self, hub: &SignalHub) {
        for module in self.modules.values_mut() {
            module.refresh(hub);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

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
        let config = Config::from_str(toml_str).unwrap();

        registry.load(&config).unwrap();
        assert_eq!(registry.left_modules().len(), 1);
    }

    #[tokio::test]
    async fn test_module_registry_attach_all() {
        let mut registry = ModuleRegistry::new();
        let toml_str = r##"
            [bar]
            [modules]
            left = [{ name = "hour", enable = true }]
            center = [{ name = "hour", enable = true }]
            right = []
        "##;
        let config = Config::from_str(toml_str).unwrap();
        registry.load(&config).unwrap();

        let (hub, _) = SignalHub::new(config);
        // This should not panic and should assign IDs 0 and 1
        registry.attach_all(&hub);
    }
}
