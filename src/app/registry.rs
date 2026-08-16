use crate::shared::config::domain::ModuleConfig;

use crate::shared::events::signals::SignalHub;
use crate::features::module_runtime::ports::{AnyModulePort, ModuleRegistryPort};
use thiserror::Error;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum ModuleError {
    #[error("Internal module error: {message}")]
    Internal { message: String },
}

use crate::shared::primitives::ModuleId;
use std::collections::HashMap;

use crate::app::builtins;

pub struct ModuleRegistry {
    modules: HashMap<ModuleId, Box<dyn AnyModulePort>>,
    left_modules: Vec<ModuleId>,
    center_modules: Vec<ModuleId>,
    right_modules: Vec<ModuleId>,
}

impl Default for ModuleRegistry {
    fn default() -> Self {
        Self::new()
    }
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
        full_config: &crate::shared::config::domain::Config,
        next_id: &mut u32,
    ) -> Result<Vec<ModuleId>, String> {
        let mut ids = Vec::new();
        for config in configs {
            if !config.is_enabled() {
                continue;
            }

            let id = ModuleId::new(*next_id);
            *next_id += 1;

            let mut module =
                builtins::BuiltinModules::find_module(config.name(), config.engine())
                    .map_err(|e| e.to_string())?;

            module.init(config, full_config).map_err(|e| e.to_string())?;
            self.modules.insert(id, module);
            ids.push(id);
        }
        Ok(ids)
    }
}

struct WatchLayoutSender {
    tx: tokio::sync::watch::Sender<
        std::collections::HashMap<crate::shared::primitives::MonitorId, crate::shared::primitives::geometry::Rect>,
    >,
}

impl crate::features::module_runtime::ports::LayoutSender for WatchLayoutSender {
    fn send_layout(&self, layout: std::collections::HashMap<crate::shared::primitives::MonitorId, crate::shared::primitives::geometry::Rect>) {
        let _ = self.tx.send(layout);
    }
}

#[async_trait::async_trait]
impl<Fact: crate::shared::rendering::ports::canvas::CanvasFactory + 'static> ModuleRegistryPort<Fact> for ModuleRegistry {
    fn left_modules(&self) -> Vec<ModuleId> {
        self.left_modules.clone()
    }

    fn center_modules(&self) -> Vec<ModuleId> {
        self.center_modules.clone()
    }

    fn right_modules(&self) -> Vec<ModuleId> {
        self.right_modules.clone()
    }

    fn load(&mut self, config: &crate::shared::config::domain::Config) -> Result<(), String> {
        self.modules.clear();
        let mut next_id = 0;

        self.left_modules = self
            .load_section(config.modules().left(), config, &mut next_id)
            .map_err(|e| e.to_string())?;
        self.center_modules = self
            .load_section(config.modules().center(), config, &mut next_id)
            .map_err(|e| e.to_string())?;
        self.right_modules = self
            .load_section(config.modules().right(), config, &mut next_id)
            .map_err(|e| e.to_string())?;

        Ok(())
    }

    fn spawn_all(
        &mut self,
        hub: std::sync::Arc<SignalHub>,
        surface_manager: crate::shared::wayland::ports::DynSurfaceManager,
        command_tx: std::sync::Arc<dyn crate::features::module_runtime::ports::CommandSender>,
        canvas_factory: std::sync::Arc<std::sync::Mutex<Fact>>,
    ) -> std::collections::HashMap<ModuleId, Box<dyn crate::features::module_runtime::ports::LayoutSender>> {
        let mut layout_senders: std::collections::HashMap<
            ModuleId,
            Box<dyn crate::features::module_runtime::ports::LayoutSender>,
        > = std::collections::HashMap::new();

        for (id, module) in self.modules.drain().collect::<Vec<_>>() {
            let (layout_tx, layout_rx) =
                tokio::sync::watch::channel(std::collections::HashMap::new());
            layout_senders.insert(id, Box::new(WatchLayoutSender { tx: layout_tx }));

            let ctx = crate::features::module_runtime::application::ModuleContext::new(
                id,
                hub.clone(),
                surface_manager.clone(),
                command_tx.clone(),
                layout_rx,
            );

            crate::features::module_runtime::application::ModuleActor::new(module, ctx, canvas_factory.clone()).spawn();
        }

        layout_senders
    }

    fn clear(&mut self) {
        self.modules.clear();
        self.left_modules.clear();
        self.center_modules.clear();
        self.right_modules.clear();
    }

    async fn register_dbus_subscriptions(&self, dbus: &mut dyn crate::shared::dbus::ports::DBusPort) {
        for module in self.modules.values() {
            for kind in module.subscriptions() {
                if let crate::shared::events::signals::SignalKind::DBus(sub) = kind
                    && let Err(e) = dbus.subscribe(sub).await
                {
                    tracing::error!("Failed to subscribe to DBus: {}", e);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::config::adapters::dto::ConfigDto;
    use crate::shared::rendering::ports::font::FontValidatorPort;

    struct MockValidator;
    impl FontValidatorPort for MockValidator {
        fn is_valid_family(&self, _family: &str) -> bool {
            true
        }
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
        let config = dto.into_domain(&MockValidator);

        crate::features::module_runtime::ports::ModuleRegistryPort::<crate::shared::rendering::adapters::tiny_skia::TinySkiaCanvasFactory>::load(&mut registry, &config).unwrap();
        assert_eq!(crate::features::module_runtime::ports::ModuleRegistryPort::<crate::shared::rendering::adapters::tiny_skia::TinySkiaCanvasFactory>::left_modules(&registry).len(), 1);
    }

    #[test]
    fn test_module_error_display() {
        let err2 = ModuleError::Internal { message: "error".into() };
        assert_eq!(err2.to_string(), "Internal module error: error");
    }

    #[test]
    fn test_watch_layout_sender() {
        use crate::features::module_runtime::ports::LayoutSender;
        let (tx, _rx) = tokio::sync::watch::channel(std::collections::HashMap::new());
        let sender = WatchLayoutSender { tx };
        
        let mut layout = std::collections::HashMap::new();
        layout.insert(crate::shared::primitives::MonitorId::new("1"), crate::shared::primitives::geometry::Rect::new(
            crate::shared::primitives::geometry::Position::new(0, 0),
            crate::shared::primitives::geometry::Size::new(0, 0)
        ));
        sender.send_layout(layout.clone());
        
        let current = _rx.borrow().clone();
        assert!(current.contains_key(&crate::shared::primitives::MonitorId::new("1")));
    }

    #[test]
    fn test_module_registry_load_errors() {
        let mut registry = ModuleRegistry::new();
        let toml_str = r##"
            [bar]
            [modules]
            left = [{ name = "nonexistent", enable = true }]
            center = []
            right = []
        "##;
        let dto: ConfigDto = toml::from_str(toml_str).unwrap();
        let config = dto.into_domain(&MockValidator);

        let result = crate::features::module_runtime::ports::ModuleRegistryPort::<crate::shared::rendering::adapters::tiny_skia::TinySkiaCanvasFactory>::load(&mut registry, &config);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn test_module_registry_clear() {
        let mut registry = ModuleRegistry::new();
        let toml_str = r##"
            [bar]
            [modules]
            left = [{ name = "hour", enable = true }]
            center = []
            right = []
        "##;
        let dto: ConfigDto = toml::from_str(toml_str).unwrap();
        let config = dto.into_domain(&MockValidator);

        crate::features::module_runtime::ports::ModuleRegistryPort::<crate::shared::rendering::adapters::tiny_skia::TinySkiaCanvasFactory>::load(&mut registry, &config).unwrap();
        
        crate::features::module_runtime::ports::ModuleRegistryPort::<crate::shared::rendering::adapters::tiny_skia::TinySkiaCanvasFactory>::clear(&mut registry);
        
        assert!(registry.left_modules.is_empty());
        assert!(registry.modules.is_empty());
    }

    #[tokio::test]
    async fn test_module_registry_register_dbus() {
        let mut registry = ModuleRegistry::new();
        let toml_str = r##"
            [bar]
            [modules]
            left = [{ name = "hour", enable = true }]
            center = []
            right = []
        "##;
        let dto: ConfigDto = toml::from_str(toml_str).unwrap();
        let config = dto.into_domain(&MockValidator);
        crate::features::module_runtime::ports::ModuleRegistryPort::<crate::shared::rendering::adapters::tiny_skia::TinySkiaCanvasFactory>::load(&mut registry, &config).unwrap();

        let mut mock_dbus = crate::shared::dbus::ports::MockDBusPort::new();
        crate::features::module_runtime::ports::ModuleRegistryPort::<crate::shared::rendering::adapters::tiny_skia::TinySkiaCanvasFactory>::register_dbus_subscriptions(&registry, &mut mock_dbus).await;
    }

    #[tokio::test]
    async fn test_module_registry_spawn_all() {
        let mut registry = ModuleRegistry::new();
        let toml_str = r##"
            [bar]
            [modules]
            left = [{ name = "hour", enable = true }]
            center = []
            right = []
        "##;
        let dto: ConfigDto = toml::from_str(toml_str).unwrap();
        let config = dto.into_domain(&MockValidator);
        crate::features::module_runtime::ports::ModuleRegistryPort::<crate::shared::rendering::adapters::tiny_skia::TinySkiaCanvasFactory>::load(&mut registry, &config).unwrap();

        let hub = std::sync::Arc::new(SignalHub::new(config.clone()));
        let surface_manager: crate::shared::wayland::ports::DynSurfaceManager = std::sync::Arc::new(crate::shared::wayland::ports::MockSurfaceManagerPort::new());
        
        struct MockSender;
        impl crate::features::module_runtime::ports::CommandSender for MockSender {
            fn send_command(&self, _cmd: crate::app::commands::AppCommand) {}
        }
        let command_tx = std::sync::Arc::new(MockSender);
        let canvas_factory = std::sync::Arc::new(std::sync::Mutex::new(crate::shared::rendering::adapters::tiny_skia::TinySkiaCanvasFactory::new()));

        let senders = crate::features::module_runtime::ports::ModuleRegistryPort::<crate::shared::rendering::adapters::tiny_skia::TinySkiaCanvasFactory>::spawn_all(
            &mut registry,
            hub,
            surface_manager,
            command_tx,
            canvas_factory
        );

        assert_eq!(senders.len(), 1);
        assert!(registry.modules.is_empty());
    }
}
