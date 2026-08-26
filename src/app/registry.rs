use crate::shared::config::domain::ModuleConfig;

use crate::features::module_runtime::ports::{AnyModulePort, ModuleRegistryPort};
use crate::shared::events::signals::SignalHub;
use thiserror::Error;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum ModuleError {
    #[error("Internal module error: {message}")]
    Internal { message: String },
}

use crate::shared::primitives::{DynamicValue, ModuleId, ModuleOptions};
use std::collections::HashMap;
use std::sync::Arc;

use crate::app::builtins;

use crate::features::styling::adapters::fs_loader::{CompositeStyleResolver, FsStyleLoader};
use crate::features::styling::adapters::lightningcss::LightningCssAdapter;
use crate::features::styling::domain::StyleSheetName;
use crate::features::styling::ports::CssParserPort;
use crate::features::styling::ports::{ParsedStyleSheetPort, StyleLoaderPort, StyleResolverPort};
use std::collections::HashSet;

pub struct ModuleRegistry {
    modules: HashMap<ModuleId, Box<dyn AnyModulePort>>,
    module_configs: HashMap<ModuleId, ModuleConfig>,
    root_module: Option<ModuleId>,
    module_ids: Vec<ModuleId>,
    module_names: HashMap<ModuleId, crate::shared::primitives::ModuleName>,
    name_to_ids: HashMap<crate::shared::primitives::ModuleName, Vec<ModuleId>>,
    dbus_subscriptions: Vec<crate::shared::dbus::domain::DBusSubscription>,
    style_to_modules: HashMap<StyleSheetName, HashSet<crate::shared::primitives::ModuleName>>,
    app_env: std::sync::Arc<crate::shared::env::domain::AppEnvironment>,
}

impl ModuleRegistry {
    #[must_use]
    pub fn new(app_env: std::sync::Arc<crate::shared::env::domain::AppEnvironment>) -> Self {
        let loader = FsStyleLoader::new(app_env.clone());
        let _ = loader.ensure_builtin_styles();

        Self {
            modules: HashMap::new(),
            module_configs: HashMap::new(),
            root_module: None,
            module_ids: Vec::new(),
            module_names: HashMap::new(),
            name_to_ids: HashMap::new(),
            dbus_subscriptions: Vec::new(),
            style_to_modules: HashMap::new(),
            app_env,
        }
    }

    pub fn clear(&mut self) {
        self.modules.clear();
        self.module_configs.clear();
        self.root_module = None;
        self.module_ids.clear();
        self.module_names.clear();
        self.name_to_ids.clear();
        self.dbus_subscriptions.clear();
        self.style_to_modules.clear();
    }

    #[must_use]
    pub fn modules_using_style(
        &self,
        sheet: &StyleSheetName,
    ) -> Vec<crate::shared::primitives::ModuleName> {
        self.style_to_modules
            .get(sheet)
            .map(|set| set.iter().cloned().collect())
            .unwrap_or_default()
    }

    #[must_use]
    pub fn create_style_resolver_for_module(
        &self,
        styles: &[StyleSheetName],
    ) -> std::sync::Arc<dyn StyleResolverPort> {
        let loader = FsStyleLoader::new(self.app_env.clone());
        let parser = LightningCssAdapter::new();
        let mut parsed_sheets: Vec<Box<dyn ParsedStyleSheetPort>> = Vec::new();

        tracing::debug!(requested_styles = ?styles.iter().map(super::super::features::styling::domain::StyleSheetName::as_str).collect::<Vec<_>>(), "Creating composite style resolver for module");

        // 1. Always load base.css first if available
        if let Ok(base_name) = StyleSheetName::new("base")
            && let Ok(base_css) = loader.load_stylesheet(&base_name)
            && let Ok(sheet) = parser.parse_stylesheet(base_name, &base_css)
        {
            parsed_sheets.push(sheet);
        }

        // 2. Load module stylesheets
        for style_name in styles {
            if style_name.as_str() != "base"
                && let Ok(css) = loader.load_stylesheet(style_name)
                && let Ok(sheet) = parser.parse_stylesheet(style_name.clone(), &css)
            {
                parsed_sheets.push(sheet);
            }
        }

        std::sync::Arc::new(CompositeStyleResolver::new(parsed_sheets))
    }

    fn load_single_module(
        &mut self,
        config: &ModuleConfig,
        full_config: &crate::shared::config::domain::Config,
        next_id: &mut u32,
    ) -> Result<ModuleId, crate::features::module_runtime::ports::RegistryLoadError> {
        use crate::app::builtins::BuiltinError;
        use crate::features::module_runtime::ports::RegistryLoadError;

        let id = ModuleId::new(*next_id);
        *next_id = next_id.saturating_add(1);

        let mut module = builtins::BuiltinModules::find_module(
            config.name(),
            config.engine(),
            &self.app_env,
        )
        .map_err(|e| match e {
            BuiltinError::ModuleNotFound { module_name, .. } => {
                RegistryLoadError::ModuleNotFound(module_name)
            }
            BuiltinError::UnsupportedEngine {
                engine,
                module_name,
            } => RegistryLoadError::UnsupportedEngine {
                engine,
                module_name,
            },
            BuiltinError::Env(e) | BuiltinError::Io(e) => RegistryLoadError::Internal(e),
        })?;

        module
            .init(config, full_config)
            .map_err(|e| RegistryLoadError::ModuleInit {
                module_name: config.name().clone(),
                source: e,
            })?;

        for sub in module.dbus_subscriptions() {
            self.dbus_subscriptions.push(sub.clone());
        }

        let mod_styles = module.styles();
        tracing::debug!(
            module = %config.name().as_str(),
            id = %id,
            styles = ?mod_styles.iter().map(super::super::features::styling::domain::StyleSheetName::as_str).collect::<Vec<_>>(),
            "Registered module style dependencies"
        );

        for style_name in mod_styles {
            self.style_to_modules
                .entry(style_name.clone())
                .or_default()
                .insert(config.name().clone());
        }

        self.module_configs.insert(id, config.clone());
        self.module_names.insert(id, config.name().clone());
        self.name_to_ids
            .entry(config.name().clone())
            .or_default()
            .push(id);
        self.module_ids.push(id);
        self.modules.insert(id, module);

        Ok(id)
    }
}

pub(crate) struct WatchLayoutSender {
    tx: tokio::sync::watch::Sender<
        std::collections::HashMap<
            crate::shared::primitives::MonitorId,
            crate::shared::primitives::geometry::Rect,
        >,
    >,
}

#[cfg(test)]
impl WatchLayoutSender {
    #[must_use]
    pub fn new(
        tx: tokio::sync::watch::Sender<
            std::collections::HashMap<
                crate::shared::primitives::MonitorId,
                crate::shared::primitives::geometry::Rect,
            >,
        >,
    ) -> Self {
        Self { tx }
    }
}

impl crate::features::module_runtime::ports::LayoutSender for WatchLayoutSender {
    fn send_layout(
        &self,
        layout: std::collections::HashMap<
            crate::shared::primitives::MonitorId,
            crate::shared::primitives::geometry::Rect,
        >,
    ) {
        let _ = self.tx.send(layout);
    }
}

#[async_trait::async_trait]
impl<Fact: crate::shared::rendering::ports::canvas::CanvasFactory + 'static>
    ModuleRegistryPort<Fact> for ModuleRegistry
{
    fn root_module(&self) -> Option<ModuleId> {
        self.root_module
    }

    fn module_ids(&self) -> &[ModuleId] {
        &self.module_ids
    }

    fn module_names(&self) -> &HashMap<ModuleId, crate::shared::primitives::ModuleName> {
        &self.module_names
    }

    fn name_to_ids(&self) -> &HashMap<crate::shared::primitives::ModuleName, Vec<ModuleId>> {
        &self.name_to_ids
    }

    fn load(
        &mut self,
        config: &crate::shared::config::domain::Config,
    ) -> Result<(), crate::features::module_runtime::ports::RegistryLoadError> {
        self.clear();
        let mut next_id = 0;

        // 1. Load the root module (default "bar")
        let root_name = config.root().name();
        let root_cfg = config.modules().get(root_name).cloned().unwrap_or_else(|| {
            ModuleConfig::new(
                root_name.clone(),
                true,
                crate::shared::config::domain::EngineSelection::Auto,
                config.root().options().clone(),
            )
        });
        let root_id = self.load_single_module(&root_cfg, config, &mut next_id)?;
        self.root_module = Some(root_id);

        // 2. Load all configured child modules
        for mod_cfg in config.modules().modules().values() {
            if mod_cfg.is_enabled() && mod_cfg.name() != root_name {
                let _ = self.load_single_module(mod_cfg, config, &mut next_id)?;
            }
        }

        // 3. Also load any modules referenced in root options (left, center, right)
        let check_sections = ["left", "center", "right"];
        for sec in check_sections {
            if let Some(DynamicValue::Array(arr)) = config.root().options().get(sec) {
                for item in arr {
                    if let Some(name_str) = item.as_str() {
                        let mod_name = crate::shared::primitives::ModuleName::new(name_str);
                        if !self.name_to_ids.contains_key(&mod_name) {
                            let auto_cfg = ModuleConfig::new(
                                mod_name.clone(),
                                true,
                                crate::shared::config::domain::EngineSelection::Auto,
                                ModuleOptions::default(),
                            );
                            let _ = self.load_single_module(&auto_cfg, config, &mut next_id)?;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn spawn_all(
        &mut self,
        hub: std::sync::Arc<SignalHub>,
        surface_manager: crate::shared::wayland::ports::DynSurfaceManager,
        command_tx: std::sync::Arc<dyn crate::features::module_runtime::ports::CommandSender>,
        canvas_factory: std::sync::Arc<std::sync::Mutex<Fact>>,
    ) -> std::collections::HashMap<
        ModuleId,
        Box<dyn crate::features::module_runtime::ports::LayoutSender>,
    > {
        let mut layout_senders: std::collections::HashMap<
            ModuleId,
            Box<dyn crate::features::module_runtime::ports::LayoutSender>,
        > = std::collections::HashMap::new();

        for (id, module) in std::mem::take(&mut self.modules) {
            let (layout_tx, layout_rx) =
                tokio::sync::watch::channel(std::collections::HashMap::new());
            layout_senders.insert(id, Box::new(WatchLayoutSender { tx: layout_tx }));

            let parent_id = if Some(id) == self.root_module {
                None
            } else {
                self.root_module
            };

            let ctx = crate::features::module_runtime::application::ModuleContext::new(
                id,
                hub.clone(),
                surface_manager.clone(),
                command_tx.clone(),
                layout_rx,
            )
            .with_parent(parent_id);

            let style_resolver = self.create_style_resolver_for_module(module.styles());

            let vdom_diff =
                Arc::new(crate::features::vdom::adapters::DefaultVdomDiffAdapter::new());

            crate::features::module_runtime::application::ModuleActor::new(
                module,
                ctx,
                canvas_factory.clone(),
                style_resolver,
                vdom_diff,
            )
            .spawn();
        }

        layout_senders
    }

    fn reload_module(
        &mut self,
        name: &crate::shared::primitives::ModuleName,
        config: &crate::shared::config::domain::Config,
        hub: std::sync::Arc<SignalHub>,
        surface_manager: crate::shared::wayland::ports::DynSurfaceManager,
        command_tx: std::sync::Arc<dyn crate::features::module_runtime::ports::CommandSender>,
        canvas_factory: std::sync::Arc<std::sync::Mutex<Fact>>,
    ) -> Result<
        std::collections::HashMap<
            ModuleId,
            Box<dyn crate::features::module_runtime::ports::LayoutSender>,
        >,
        crate::features::module_runtime::ports::RegistryLoadError,
    > {
        use crate::app::builtins::BuiltinError;
        use crate::features::module_runtime::ports::RegistryLoadError;

        let mut new_senders: std::collections::HashMap<
            ModuleId,
            Box<dyn crate::features::module_runtime::ports::LayoutSender>,
        > = std::collections::HashMap::new();
        let target_ids: Vec<ModuleId> = self
            .module_configs
            .iter()
            .filter(|(_, cfg)| cfg.name() == name)
            .map(|(id, _)| *id)
            .collect();

        for id in target_ids {
            let Some(cfg) = self.module_configs.get(&id) else {
                continue;
            };
            let mut module =
                builtins::BuiltinModules::find_module(cfg.name(), cfg.engine(), &self.app_env)
                    .map_err(|e| match e {
                        BuiltinError::ModuleNotFound { module_name, .. } => {
                            RegistryLoadError::ModuleNotFound(module_name)
                        }
                        BuiltinError::UnsupportedEngine {
                            engine,
                            module_name,
                        } => RegistryLoadError::UnsupportedEngine {
                            engine,
                            module_name,
                        },
                        BuiltinError::Env(e) | BuiltinError::Io(e) => {
                            RegistryLoadError::Internal(e)
                        }
                    })?;

            module
                .init(cfg, config)
                .map_err(|e| RegistryLoadError::ModuleInit {
                    module_name: cfg.name().clone(),
                    source: e,
                })?;

            for style_name in module.styles() {
                self.style_to_modules
                    .entry(style_name.clone())
                    .or_default()
                    .insert(cfg.name().clone());
            }

            let (layout_tx, layout_rx) = tokio::sync::watch::channel(HashMap::new());
            new_senders.insert(id, Box::new(WatchLayoutSender { tx: layout_tx }));

            let ctx = crate::features::module_runtime::application::ModuleContext::new(
                id,
                hub.clone(),
                surface_manager.clone(),
                command_tx.clone(),
                layout_rx,
            );

            let style_resolver = self.create_style_resolver_for_module(module.styles());
            let vdom_diff =
                Arc::new(crate::features::vdom::adapters::DefaultVdomDiffAdapter::new());

            crate::features::module_runtime::application::ModuleActor::new(
                module,
                ctx,
                canvas_factory.clone(),
                style_resolver,
                vdom_diff,
            )
            .spawn();
        }

        Ok(new_senders)
    }

    fn modules_using_style(
        &self,
        sheet: &StyleSheetName,
    ) -> Vec<crate::shared::primitives::ModuleName> {
        self.modules_using_style(sheet)
    }

    fn clear(&mut self) {
        self.clear();
    }

    async fn register_dbus_subscriptions(
        &self,
        dbus: &mut crate::shared::dbus::subscription_manager::DbusSubscriptionManager,
    ) {
        for sub in &self.dbus_subscriptions {
            if let Err(e) = dbus.subscribe(sub.clone()).await {
                tracing::error!("Failed to subscribe to DBus: {e}");
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

    struct MockSender;
    impl crate::features::module_runtime::ports::CommandSender for MockSender {
        fn send_command(&self, _cmd: crate::app::commands::AppCommand) {}
    }

    #[test]
    fn test_module_registry_load() {
        let app_env = std::sync::Arc::new(crate::shared::env::domain::AppEnvironment::new(
            crate::shared::env::domain::HomeDir::new(std::path::PathBuf::from("/tmp")),
            crate::shared::env::domain::XdgCacheHome::new(std::path::PathBuf::from("/tmp")),
            crate::shared::env::domain::XdgRuntimeDir::new(std::path::PathBuf::from("/tmp")),
            crate::shared::env::domain::RustLog::new(String::new()),
            None,
        ));
        let mut registry = ModuleRegistry::new(app_env);
        let toml_str = r#"
            [root]
            name = "bar"
            left = ["hour"]
        "#;
        let dto: ConfigDto = toml::from_str(toml_str).unwrap();
        let config = dto.into_domain(&MockValidator);

        crate::features::module_runtime::ports::ModuleRegistryPort::<
            crate::shared::rendering::adapters::tiny_skia::TinySkiaCanvasFactory,
        >::load(&mut registry, &config)
        .unwrap();
        assert_eq!(
            crate::features::module_runtime::ports::ModuleRegistryPort::<
                crate::shared::rendering::adapters::tiny_skia::TinySkiaCanvasFactory,
            >::module_ids(&registry)
            .len(),
            2
        );
        assert!(
            crate::features::module_runtime::ports::ModuleRegistryPort::<
                crate::shared::rendering::adapters::tiny_skia::TinySkiaCanvasFactory,
            >::root_module(&registry)
            .is_some()
        );
    }

    #[test]
    fn test_module_error_display() {
        let err2 = ModuleError::Internal {
            message: "error".into(),
        };
        assert_eq!(err2.to_string(), "Internal module error: error");
    }

    #[test]
    fn test_watch_layout_sender() {
        use crate::features::module_runtime::ports::LayoutSender;
        let (tx, rx) = tokio::sync::watch::channel(std::collections::HashMap::new());
        let sender = WatchLayoutSender { tx };

        let mut layout = std::collections::HashMap::new();
        layout.insert(
            crate::shared::primitives::MonitorId::new("1"),
            crate::shared::primitives::geometry::Rect::new(
                crate::shared::primitives::geometry::Position::new(0, 0),
                crate::shared::primitives::geometry::Size::new(0, 0),
            ),
        );
        sender.send_layout(layout.clone());

        let current = rx.borrow().clone();
        assert!(current.contains_key(&crate::shared::primitives::MonitorId::new("1")));
    }

    #[test]
    fn test_module_registry_load_errors() {
        let app_env = std::sync::Arc::new(crate::shared::env::domain::AppEnvironment::new(
            crate::shared::env::domain::HomeDir::new(std::path::PathBuf::from("/tmp")),
            crate::shared::env::domain::XdgCacheHome::new(std::path::PathBuf::from("/tmp")),
            crate::shared::env::domain::XdgRuntimeDir::new(std::path::PathBuf::from("/tmp")),
            crate::shared::env::domain::RustLog::new(String::new()),
            None,
        ));
        let mut registry = ModuleRegistry::new(app_env);
        let toml_str = r#"
            [root]
            name = "bar"
            left = ["nonexistent"]
        "#;
        let dto: ConfigDto = toml::from_str(toml_str).unwrap();
        let config = dto.into_domain(&MockValidator);

        let result = crate::features::module_runtime::ports::ModuleRegistryPort::<
            crate::shared::rendering::adapters::tiny_skia::TinySkiaCanvasFactory,
        >::load(&mut registry, &config);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            crate::features::module_runtime::ports::RegistryLoadError::ModuleNotFound(_)
        ));
    }

    #[test]
    fn test_module_registry_clear() {
        let app_env = std::sync::Arc::new(crate::shared::env::domain::AppEnvironment::new(
            crate::shared::env::domain::HomeDir::new(std::path::PathBuf::from("/tmp")),
            crate::shared::env::domain::XdgCacheHome::new(std::path::PathBuf::from("/tmp")),
            crate::shared::env::domain::XdgRuntimeDir::new(std::path::PathBuf::from("/tmp")),
            crate::shared::env::domain::RustLog::new(String::new()),
            None,
        ));
        let mut registry = ModuleRegistry::new(app_env);
        let toml_str = r#"
            [root]
            name = "bar"
            left = ["hour"]
        "#;
        let dto: ConfigDto = toml::from_str(toml_str).unwrap();
        let config = dto.into_domain(&MockValidator);

        crate::features::module_runtime::ports::ModuleRegistryPort::<
            crate::shared::rendering::adapters::tiny_skia::TinySkiaCanvasFactory,
        >::load(&mut registry, &config)
        .unwrap();

        crate::features::module_runtime::ports::ModuleRegistryPort::<
            crate::shared::rendering::adapters::tiny_skia::TinySkiaCanvasFactory,
        >::clear(&mut registry);

        assert!(registry.module_ids.is_empty());
        assert!(registry.modules.is_empty());
    }

    #[tokio::test]
    async fn test_module_registry_register_dbus() {
        let app_env = std::sync::Arc::new(crate::shared::env::domain::AppEnvironment::new(
            crate::shared::env::domain::HomeDir::new(std::path::PathBuf::from("/tmp")),
            crate::shared::env::domain::XdgCacheHome::new(std::path::PathBuf::from("/tmp")),
            crate::shared::env::domain::XdgRuntimeDir::new(std::path::PathBuf::from("/tmp")),
            crate::shared::env::domain::RustLog::new(String::new()),
            None,
        ));
        let mut registry = ModuleRegistry::new(app_env);
        let toml_str = r#"
            [root]
            name = "bar"
            left = ["hour"]
        "#;
        let dto: ConfigDto = toml::from_str(toml_str).unwrap();
        let config = dto.into_domain(&MockValidator);
        crate::features::module_runtime::ports::ModuleRegistryPort::<
            crate::shared::rendering::adapters::tiny_skia::TinySkiaCanvasFactory,
        >::load(&mut registry, &config)
        .unwrap();

        let hub = std::sync::Arc::new(crate::shared::events::signals::SignalHub::new(
            config.clone(),
        ));
        let mock_conn = crate::shared::dbus::ports::MockDbusConnectionPort::new();
        let mut mock_dbus = crate::shared::dbus::subscription_manager::DbusSubscriptionManager::new(
            std::sync::Arc::new(mock_conn),
            &hub,
        );
        crate::features::module_runtime::ports::ModuleRegistryPort::<
            crate::shared::rendering::adapters::tiny_skia::TinySkiaCanvasFactory,
        >::register_dbus_subscriptions(&registry, &mut mock_dbus)
        .await;
    }

    #[tokio::test]
    async fn test_module_registry_spawn_all() {
        let app_env = std::sync::Arc::new(crate::shared::env::domain::AppEnvironment::new(
            crate::shared::env::domain::HomeDir::new(std::path::PathBuf::from("/tmp")),
            crate::shared::env::domain::XdgCacheHome::new(std::path::PathBuf::from("/tmp")),
            crate::shared::env::domain::XdgRuntimeDir::new(std::path::PathBuf::from("/tmp")),
            crate::shared::env::domain::RustLog::new(String::new()),
            None,
        ));
        let mut registry = ModuleRegistry::new(app_env);
        let toml_str = r#"
            [root]
            name = "bar"
            left = ["hour"]
        "#;
        let dto: ConfigDto = toml::from_str(toml_str).unwrap();
        let config = dto.into_domain(&MockValidator);
        crate::features::module_runtime::ports::ModuleRegistryPort::<
            crate::shared::rendering::adapters::tiny_skia::TinySkiaCanvasFactory,
        >::load(&mut registry, &config)
        .unwrap();

        let hub = std::sync::Arc::new(SignalHub::new(config.clone()));
        let surface_manager: crate::shared::wayland::ports::DynSurfaceManager =
            std::sync::Arc::new(crate::shared::wayland::ports::MockSurfaceManagerPort::new());

        let command_tx = std::sync::Arc::new(MockSender);
        let canvas_factory = std::sync::Arc::new(std::sync::Mutex::new(
            crate::shared::rendering::adapters::tiny_skia::TinySkiaCanvasFactory::new(),
        ));

        let senders = crate::features::module_runtime::ports::ModuleRegistryPort::<
            crate::shared::rendering::adapters::tiny_skia::TinySkiaCanvasFactory,
        >::spawn_all(
            &mut registry,
            hub,
            surface_manager,
            command_tx,
            canvas_factory,
        );

        assert_eq!(senders.len(), 2); // bar + hour
        assert!(registry.modules.is_empty());
    }
}
