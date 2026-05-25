use crate::domain::config::ModuleConfig;
use crate::ports::canvas::Canvas;
use crate::domain::signals::{SignalHub, SignalKind};
use crate::domain::errors::DomainError;
use crate::domain::{ModuleId, MonitorId, geometry::Size};
use std::collections::HashMap;

pub mod lua;
pub mod rhai;

/// A type-erased version of CrankyModule.
pub trait AnyModule: Send + Sync {
    fn init(
        &mut self,
        config: &ModuleConfig,
        bar_config: &crate::domain::config::BarConfig,
    ) -> Result<(), DomainError>;

    fn subscriptions(&self) -> Vec<SignalKind>;

    fn refresh(&mut self, hub: &SignalHub);

    fn view(&self, canvas: &mut dyn Canvas, monitor: &MonitorId);

    fn measure(&self, canvas: &mut dyn Canvas, monitor: &MonitorId) -> Size;

    fn on_event(&mut self, event: crate::domain::events::InputEvent) -> Vec<crate::domain::commands::AppCommand>;
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

    pub fn get_mut(&mut self, id: ModuleId) -> Option<&mut dyn AnyModule> {
        self.modules.get_mut(&id).map(|m| &mut **m as &mut dyn AnyModule)
    }

    pub fn load(&mut self, config: &crate::domain::config::Config) -> Result<(), DomainError> {
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
        bar_config: &crate::domain::config::BarConfig,
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
                "hour" => Box::new(lua::LuaModule::built_in("hour")
                    .ok_or_else(|| DomainError::ModuleNotFound { module_name: "hour".to_string() })?),
                "workspace" => Box::new(lua::LuaModule::built_in("workspace")
                    .ok_or_else(|| DomainError::ModuleNotFound { module_name: "workspace".to_string() })?),
                "applet" => Box::new(lua::LuaModule::built_in("applet")
                    .ok_or_else(|| DomainError::ModuleNotFound { module_name: "applet".to_string() })?),
                name => {
                    // Try to load as lua first, then rhai
                    if let Some(m) = lua::LuaModule::external(name) {
                        Box::new(m)
                    } else if let Some(m) = rhai::RhaiModule::external(name) {
                        Box::new(m)
                    } else {
                        return Err(DomainError::ModuleNotFound { module_name: name.to_string() });
                    }
                }
            };

            module.init(config, bar_config)?;
            self.modules.insert(id, module);
            ids.push(id);
        }
        Ok(ids)
    }

    pub fn attach_all(&mut self, hub: &SignalHub) {
        for (id, module) in self.modules.iter() {
            let subs = module.subscriptions();
            for kind in subs {
                match kind {
                    SignalKind::Time => {
                        let mut rx = hub.time_rx();
                        let tx = hub.dirty_tx();
                        let id = *id;
                        tokio::spawn(async move {
                            while rx.changed().await.is_ok() {
                                let _ = tx.send(id).await;
                            }
                        });
                    }
                    SignalKind::Hyprland => {
                        let mut rx = hub.hyprland_rx();
                        let tx = hub.dirty_tx();
                        let id = *id;
                        tokio::spawn(async move {
                            while rx.changed().await.is_ok() {
                                let _ = tx.send(id).await;
                            }
                        });
                    }
                    SignalKind::DBus(_) => {
                        // Handled centrally in run via register_dbus_subscriptions
                    }
                    SignalKind::Applets => {
                        let mut rx = hub.applets_rx();
                        let tx = hub.dirty_tx();
                        let id = *id;
                        tokio::spawn(async move {
                            while rx.changed().await.is_ok() {
                                let _ = tx.send(id).await;
                            }
                        });
                    }
                    SignalKind::Metrics => {
                        let mut rx = hub.metrics_rx();
                        let tx = hub.dirty_tx();
                        let id = *id;
                        tokio::spawn(async move {
                            while rx.changed().await.is_ok() {
                                let _ = tx.send(id).await;
                            }
                        });
                    }
                }
            }
        }
    }

    pub async fn register_dbus_subscriptions(&self, dbus: &mut impl crate::ports::DBusPort) {
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

    pub fn refresh_all(&mut self, hub: &SignalHub) {
        for module in self.modules.values_mut() {
            module.refresh(hub);
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
        let dto: ConfigDto = toml::from_str(toml_str).unwrap();
        let config = dto.to_domain(&MockValidator);
        registry.load(&config).unwrap();

        let (hub, _) = SignalHub::new(config);
        // This should not panic and should assign IDs 0 and 1
        registry.attach_all(&hub);
    }
}
