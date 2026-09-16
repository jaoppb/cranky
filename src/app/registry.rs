use crate::shared::config::domain::ModuleConfig;

use crate::features::module_runtime::ports::{AnyModulePort, ModuleRegistryPort};
pub use crate::shared::scripting::ports::ModuleError;

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
    /// This module's full identity (name + instance) — `module_names` alone
    /// can't tell two same-named sites apart.
    module_keys: HashMap<ModuleId, crate::shared::primitives::ModuleKey>,
    /// The real parent recorded when this site was loaded, read by
    /// `spawn_all` instead of assuming every non-root module is root's
    /// child.
    module_parents: HashMap<ModuleId, Option<ModuleId>>,
    /// `(parent, key) -> id`, so a `ContainerLayoutsCalculated` event naming
    /// its own parent and a child's key resolves to the one actor embedded
    /// at that exact site, not just any actor sharing that name.
    site_index: HashMap<crate::shared::primitives::ModuleSite, ModuleId>,
    /// Monotonic, never reused for the registry's lifetime — unlike the old
    /// per-`load()` local counter, this survives a single lazy spawn between
    /// full config reloads, so a lazily-minted `ModuleId` can never collide
    /// with a config-declared one.
    next_module_id: u32,
    dbus_subscriptions: Vec<crate::shared::dbus::domain::DBusSubscription>,
    style_to_modules: HashMap<StyleSheetName, HashSet<crate::shared::primitives::ModuleName>>,
    active_signals: HashSet<crate::shared::events::signals::SignalKind>,
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
            module_keys: HashMap::new(),
            module_parents: HashMap::new(),
            site_index: HashMap::new(),
            next_module_id: 0,
            dbus_subscriptions: Vec::new(),
            style_to_modules: HashMap::new(),
            active_signals: HashSet::new(),
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
        self.module_keys.clear();
        self.module_parents.clear();
        self.site_index.clear();
        self.next_module_id = 0;
        self.dbus_subscriptions.clear();
        self.style_to_modules.clear();
        self.active_signals.clear();
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
        parent_id: Option<ModuleId>,
        instance_id: Option<crate::shared::primitives::ModuleInstanceId>,
    ) -> Result<
        (ModuleId, Box<dyn AnyModulePort>),
        crate::features::module_runtime::ports::RegistryLoadError,
    > {
        use crate::app::builtins::BuiltinError;
        use crate::features::module_runtime::ports::RegistryLoadError;

        let id = ModuleId::new(self.next_module_id);
        self.next_module_id = self.next_module_id.saturating_add(1);

        let mut module =
            builtins::BuiltinModules::find_module(config.name(), config.engine(), &self.app_env)
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

        for sub in module.subscriptions() {
            self.active_signals.insert(*sub);
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
        let key = crate::shared::primitives::ModuleKey::new(config.name().clone(), instance_id);
        self.module_keys.insert(id, key.clone());
        self.module_parents.insert(id, parent_id);
        self.site_index.insert(
            crate::shared::primitives::ModuleSite::new(parent_id, key),
            id,
        );
        self.module_ids.push(id);

        Ok((id, module))
    }

    /// Mints a `ModuleId` for a site with no config stanza — a `Module` node
    /// resolved lazily instead of declared under `[modules.*]` or
    /// `root.{left,center,right}` — and spawns its actor immediately rather
    /// than waiting for the next `spawn_all`. `find_module`/`init` failure is
    /// returned to the caller instead of being fatal: unlike `load()`, a
    /// lazy spawn happens mid-session, in the middle of some other module's
    /// render, and must not bring the bar down.
    fn spawn_module_lazy<
        Fact: crate::shared::rendering::ports::canvas::CanvasFactory + 'static,
        LS: crate::features::module_runtime::ports::LayoutEventSender + 'static,
        DS: crate::features::layout_engine::domain::DisplayCommandSender + 'static,
        US: crate::features::vdom::domain::UiCommandSender + 'static,
    >(
        &mut self,
        parent_id: ModuleId,
        name: &crate::shared::primitives::ModuleName,
        instance_id: Option<crate::shared::primitives::ModuleInstanceId>,
        options: ModuleOptions,
        full_config: &crate::shared::config::domain::Config,
        deps: &crate::features::module_runtime::ports::ModuleRuntimeDependencies<Fact, LS, DS, US>,
    ) -> Result<
        crate::features::module_runtime::ports::SpawnedModule,
        crate::features::module_runtime::ports::RegistryLoadError,
    > {
        let cfg = ModuleConfig::new(
            name.clone(),
            true,
            crate::shared::config::domain::EngineSelection::Auto,
            options,
        );
        let (id, module) = self.load_single_module(&cfg, full_config, Some(parent_id), instance_id)?;

        let (layout_tx, layout_rx) = tokio::sync::watch::channel(HashMap::new());
        let sender: Box<dyn crate::features::module_runtime::ports::LayoutSender> =
            Box::new(WatchLayoutSender { tx: layout_tx });

        let ctx = crate::features::module_runtime::application::ModuleContext::new(
            id,
            deps.hub.clone(),
            deps.surface_manager.clone(),
            deps.layout_sender.clone(),
            deps.display_sender.clone(),
            deps.ui_sender.clone(),
            layout_rx,
        )
        .with_parent(Some(parent_id));

        let style_resolver = self.create_style_resolver_for_module(module.styles());
        let vdom_diff = Arc::new(crate::features::vdom::adapters::DefaultVdomDiffAdapter::new());

        crate::features::module_runtime::application::ModuleActor::new(
            module,
            ctx,
            deps.canvas_factory.clone(),
            style_resolver,
            vdom_diff,
        )
        .spawn();

        Ok(crate::features::module_runtime::ports::SpawnedModule::new(
            id, sender,
        ))
    }
}

pub(crate) struct WatchLayoutSender {
    tx: tokio::sync::watch::Sender<
        std::collections::HashMap<
            crate::shared::primitives::MonitorId,
            crate::shared::primitives::ChildBounds,
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
                crate::shared::primitives::ChildBounds,
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
            crate::shared::primitives::ChildBounds,
        >,
    ) {
        let _ = self.tx.send(layout);
    }
}

#[async_trait::async_trait]
impl<
    Fact: crate::shared::rendering::ports::canvas::CanvasFactory + 'static,
    LS: crate::features::module_runtime::ports::LayoutEventSender + 'static,
    DS: crate::features::layout_engine::domain::DisplayCommandSender + 'static,
    US: crate::features::vdom::domain::UiCommandSender + 'static,
> ModuleRegistryPort<Fact, LS, DS, US> for ModuleRegistry
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

    fn module_keys(&self) -> &HashMap<ModuleId, crate::shared::primitives::ModuleKey> {
        &self.module_keys
    }

    fn resolve_site(
        &self,
        parent: Option<ModuleId>,
        key: &crate::shared::primitives::ModuleKey,
    ) -> Option<ModuleId> {
        self.site_index
            .get(&crate::shared::primitives::ModuleSite::new(
                parent,
                key.clone(),
            ))
            .copied()
    }

    fn parent_of(&self, id: ModuleId) -> Option<ModuleId> {
        self.module_parents.get(&id).copied().flatten()
    }

    fn spawn_module(
        &mut self,
        parent: ModuleId,
        name: &crate::shared::primitives::ModuleName,
        instance_id: Option<crate::shared::primitives::ModuleInstanceId>,
        options: ModuleOptions,
        config: &crate::shared::config::domain::Config,
        deps: &crate::features::module_runtime::ports::ModuleRuntimeDependencies<Fact, LS, DS, US>,
    ) -> Result<
        crate::features::module_runtime::ports::SpawnedModule,
        crate::features::module_runtime::ports::RegistryLoadError,
    > {
        self.spawn_module_lazy(parent, name, instance_id, options, config, deps)
    }

    fn load(
        &mut self,
        config: &crate::shared::config::domain::Config,
    ) -> Result<(), crate::features::module_runtime::ports::RegistryLoadError> {
        self.clear();

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
        let (root_id, root_module) = self.load_single_module(&root_cfg, config, None, None)?;
        self.modules.insert(root_id, root_module);
        self.root_module = Some(root_id);

        // 2. Load all configured child modules
        for mod_cfg in config.modules().modules().values() {
            if mod_cfg.is_enabled() && mod_cfg.name() != root_name {
                let (id, module) =
                    self.load_single_module(mod_cfg, config, Some(root_id), None)?;
                self.modules.insert(id, module);
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
                            let (id, module) = self.load_single_module(
                                &auto_cfg,
                                config,
                                Some(root_id),
                                None,
                            )?;
                            self.modules.insert(id, module);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn spawn_all(
        &mut self,
        deps: &crate::features::module_runtime::ports::ModuleRuntimeDependencies<Fact, LS, DS, US>,
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

            let parent_id = self.module_parents.get(&id).copied().flatten();

            let ctx = crate::features::module_runtime::application::ModuleContext::new(
                id,
                deps.hub.clone(),
                deps.surface_manager.clone(),
                deps.layout_sender.clone(),
                deps.display_sender.clone(),
                deps.ui_sender.clone(),
                layout_rx,
            )
            .with_parent(parent_id);

            let style_resolver = self.create_style_resolver_for_module(module.styles());

            let vdom_diff =
                Arc::new(crate::features::vdom::adapters::DefaultVdomDiffAdapter::new());

            crate::features::module_runtime::application::ModuleActor::new(
                module,
                ctx,
                deps.canvas_factory.clone(),
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
        deps: &crate::features::module_runtime::ports::ModuleRuntimeDependencies<Fact, LS, DS, US>,
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

            for sub in module.subscriptions() {
                self.active_signals.insert(*sub);
            }

            let (layout_tx, layout_rx) = tokio::sync::watch::channel(HashMap::new());
            new_senders.insert(id, Box::new(WatchLayoutSender { tx: layout_tx }));

            let parent_id = self.module_parents.get(&id).copied().flatten();

            let ctx = crate::features::module_runtime::application::ModuleContext::new(
                id,
                deps.hub.clone(),
                deps.surface_manager.clone(),
                deps.layout_sender.clone(),
                deps.display_sender.clone(),
                deps.ui_sender.clone(),
                layout_rx,
            )
            .with_parent(parent_id);

            let style_resolver = self.create_style_resolver_for_module(module.styles());
            let vdom_diff =
                Arc::new(crate::features::vdom::adapters::DefaultVdomDiffAdapter::new());

            crate::features::module_runtime::application::ModuleActor::new(
                module,
                ctx,
                deps.canvas_factory.clone(),
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

    fn active_signal_subscriptions(
        &self,
    ) -> &std::collections::HashSet<crate::shared::events::signals::SignalKind> {
        &self.active_signals
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::config::adapters::dto::ConfigDto;
    use crate::shared::events::signals::SignalHub;
    use crate::shared::rendering::ports::font::FontValidatorPort;

    struct MockValidator;
    impl FontValidatorPort for MockValidator {
        fn is_valid_family(&self, _family: &str) -> bool {
            true
        }
    }

    type TestRegistryPort = dyn ModuleRegistryPort<
            crate::shared::rendering::adapters::tiny_skia::TinySkiaCanvasFactory,
            crate::features::module_runtime::test_support::MockLayoutSender,
            crate::features::module_runtime::test_support::MockDisplaySender,
            crate::features::module_runtime::test_support::MockUiSender,
        >;

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
            left = ["clock"]
        "#;
        let dto: ConfigDto = toml::from_str(toml_str).unwrap();
        let config = dto.into_domain(&MockValidator);

        TestRegistryPort::load(&mut registry, &config).unwrap();
        assert_eq!(TestRegistryPort::module_ids(&registry).len(), 2);
        assert!(TestRegistryPort::root_module(&registry).is_some());
    }

    #[test]
    fn test_per_site_module_identity() {
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
        "#;
        let dto: ConfigDto = toml::from_str(toml_str).unwrap();
        let config = dto.into_domain(&MockValidator);

        let calendar_cfg = ModuleConfig::new(
            crate::shared::primitives::ModuleName::new("calendar"),
            true,
            crate::shared::config::domain::EngineSelection::Auto,
            crate::shared::primitives::ModuleOptions::default(),
        );

        let parent_a = ModuleId::new(100);
        let parent_b = ModuleId::new(200);

        let (id_a, _) = registry
            .load_single_module(&calendar_cfg, &config, Some(parent_a), None)
            .unwrap();
        let (id_b, _) = registry
            .load_single_module(&calendar_cfg, &config, Some(parent_b), None)
            .unwrap();

        // Two sites embedding the same module name mint distinct ModuleIds.
        assert_ne!(id_a, id_b);

        let key = crate::shared::primitives::ModuleKey::from_name(
            crate::shared::primitives::ModuleName::new("calendar"),
        );

        // Each site resolves to its own actor, keyed by its own parent —
        // never to the other parent's child of the same name.
        assert_eq!(
            TestRegistryPort::resolve_site(&registry, Some(parent_a), &key),
            Some(id_a)
        );
        assert_eq!(
            TestRegistryPort::resolve_site(&registry, Some(parent_b), &key),
            Some(id_b)
        );
        assert_eq!(
            TestRegistryPort::resolve_site(&registry, Some(ModuleId::new(999)), &key),
            None
        );

        assert_eq!(registry.module_parents.get(&id_a).copied().flatten(), Some(parent_a));
        assert_eq!(registry.module_parents.get(&id_b).copied().flatten(), Some(parent_b));
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
            crate::shared::primitives::ChildBounds::new(
                crate::shared::primitives::geometry::Rect::new(
                    crate::shared::primitives::geometry::Position::new(0, 0),
                    crate::shared::primitives::geometry::Size::new(0, 0),
                ),
                crate::shared::primitives::SizeConstraint::none(),
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

        let result = TestRegistryPort::load(&mut registry, &config);
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
            left = ["clock"]
        "#;
        let dto: ConfigDto = toml::from_str(toml_str).unwrap();
        let config = dto.into_domain(&MockValidator);

        TestRegistryPort::load(&mut registry, &config).unwrap();
        TestRegistryPort::clear(&mut registry);

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
            left = ["clock"]
        "#;
        let dto: ConfigDto = toml::from_str(toml_str).unwrap();
        let config = dto.into_domain(&MockValidator);
        TestRegistryPort::load(&mut registry, &config).unwrap();

        let hub = std::sync::Arc::new(crate::shared::events::signals::SignalHub::new(
            config.clone(),
        ));
        let mock_conn = crate::shared::dbus::ports::MockDbusConnectionPort::new();
        let mut mock_dbus = crate::shared::dbus::subscription_manager::DbusSubscriptionManager::new(
            std::sync::Arc::new(mock_conn),
            &hub,
        );
        TestRegistryPort::register_dbus_subscriptions(&registry, &mut mock_dbus).await;
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
            left = ["clock"]
        "#;
        let dto: ConfigDto = toml::from_str(toml_str).unwrap();
        let config = dto.into_domain(&MockValidator);

        let hub = std::sync::Arc::new(SignalHub::new(config.clone()));
        let surface_manager: crate::shared::wayland::ports::DynSurfaceManager =
            std::sync::Arc::new(crate::shared::wayland::ports::MockSurfaceManagerPort::new());

        let layout_sender = std::sync::Arc::new(|_| ());
        let display_sender = std::sync::Arc::new(|_| ());
        let ui_sender = std::sync::Arc::new(|_| ());
        let canvas_factory =
            crate::shared::rendering::adapters::tiny_skia::TinySkiaCanvasFactory::new();

        let deps = crate::features::module_runtime::ports::ModuleRuntimeDependencies::new(
            hub,
            surface_manager,
            layout_sender,
            display_sender,
            ui_sender,
            canvas_factory,
        );

        TestRegistryPort::load(&mut registry, &config).unwrap();
        let senders = registry.spawn_all(&deps);

        assert_eq!(senders.len(), 2); // bar + clock
        assert!(registry.modules.is_empty());
        assert!(
            TestRegistryPort::active_signal_subscriptions(&registry)
                .contains(&crate::shared::events::signals::SignalKind::Time)
        );
    }

    #[tokio::test]
    async fn test_spawn_module_lazy_mints_a_working_site() {
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
        "#;
        let dto: ConfigDto = toml::from_str(toml_str).unwrap();
        let config = dto.into_domain(&MockValidator);
        TestRegistryPort::load(&mut registry, &config).unwrap();
        let root_id = TestRegistryPort::root_module(&registry).unwrap();

        let hub = std::sync::Arc::new(SignalHub::new(config.clone()));
        let surface_manager: crate::shared::wayland::ports::DynSurfaceManager =
            std::sync::Arc::new(crate::shared::wayland::ports::MockSurfaceManagerPort::new());
        let layout_sender = std::sync::Arc::new(|_| ());
        let display_sender = std::sync::Arc::new(|_| ());
        let ui_sender = std::sync::Arc::new(|_| ());
        let canvas_factory =
            crate::shared::rendering::adapters::tiny_skia::TinySkiaCanvasFactory::new();
        let deps = crate::features::module_runtime::ports::ModuleRuntimeDependencies::new(
            hub,
            surface_manager,
            layout_sender,
            display_sender,
            ui_sender,
            canvas_factory,
        );

        let name = crate::shared::primitives::ModuleName::new("calendar");
        let spawned = registry
            .spawn_module(
                root_id,
                &name,
                None,
                crate::shared::primitives::ModuleOptions::default(),
                &config,
                &deps,
            )
            .unwrap();
        let id = spawned.id();

        assert_ne!(id, root_id);
        let key = crate::shared::primitives::ModuleKey::from_name(name);
        assert_eq!(
            TestRegistryPort::resolve_site(&registry, Some(root_id), &key),
            Some(id)
        );
        assert_eq!(TestRegistryPort::parent_of(&registry, id), Some(root_id));
        // The lazily spawned module was handed off to its own actor task,
        // not left sitting in the pre-spawn pool.
        assert!(!registry.modules.contains_key(&id));
    }

    #[test]
    fn test_spawn_module_lazy_propagates_not_found() {
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
        "#;
        let dto: ConfigDto = toml::from_str(toml_str).unwrap();
        let config = dto.into_domain(&MockValidator);
        TestRegistryPort::load(&mut registry, &config).unwrap();
        let root_id = TestRegistryPort::root_module(&registry).unwrap();

        let hub = std::sync::Arc::new(SignalHub::new(config.clone()));
        let surface_manager: crate::shared::wayland::ports::DynSurfaceManager =
            std::sync::Arc::new(crate::shared::wayland::ports::MockSurfaceManagerPort::new());
        let layout_sender = std::sync::Arc::new(|_| ());
        let display_sender = std::sync::Arc::new(|_| ());
        let ui_sender = std::sync::Arc::new(|_| ());
        let canvas_factory =
            crate::shared::rendering::adapters::tiny_skia::TinySkiaCanvasFactory::new();
        let deps = crate::features::module_runtime::ports::ModuleRuntimeDependencies::new(
            hub,
            surface_manager,
            layout_sender,
            display_sender,
            ui_sender,
            canvas_factory,
        );

        let name = crate::shared::primitives::ModuleName::new("does-not-exist");
        let result = registry.spawn_module(
            root_id,
            &name,
            None,
            crate::shared::primitives::ModuleOptions::default(),
            &config,
            &deps,
        );
        assert!(matches!(
            result,
            Err(crate::features::module_runtime::ports::RegistryLoadError::ModuleNotFound(_))
        ));
    }
}
