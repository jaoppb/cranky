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

/// One module site's durable facts: its full key (name + instance), the
/// parent it's embedded under, and the `LayoutSurface` it was spawned onto.
///
/// Replaces the old `module_names` + `module_keys` + `module_parents` trio
/// of parallel per-`ModuleId` maps, which every write site had to keep in
/// lockstep by hand, and which had no durable place to remember a lazily
/// spawned site's `LayoutSurface` — so a reload of an embedded module (see
/// `reload_module`) silently reset it back to `LayoutSurface::Bar`.
#[derive(Debug, Clone)]
pub struct ModuleSiteInfo {
    key: crate::shared::primitives::ModuleKey,
    parent: Option<ModuleId>,
    surface: crate::shared::primitives::LayoutSurface,
}

impl ModuleSiteInfo {
    #[must_use]
    pub const fn key(&self) -> &crate::shared::primitives::ModuleKey {
        &self.key
    }

    #[must_use]
    pub const fn parent(&self) -> Option<ModuleId> {
        self.parent
    }

    #[must_use]
    pub const fn surface(&self) -> crate::shared::primitives::LayoutSurface {
        self.surface
    }
}

pub struct ModuleRegistry {
    modules: HashMap<ModuleId, Box<dyn AnyModulePort>>,
    module_configs: HashMap<ModuleId, ModuleConfig>,
    root_module: Option<ModuleId>,
    module_ids: Vec<ModuleId>,
    name_to_ids: HashMap<crate::shared::primitives::ModuleName, Vec<ModuleId>>,
    /// Every module site's durable facts, keyed by its `ModuleId`. The
    /// single source of truth for a site's key, parent and surface across
    /// its whole lifetime — see `ModuleSiteInfo`.
    sites: HashMap<ModuleId, ModuleSiteInfo>,
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
            name_to_ids: HashMap::new(),
            sites: HashMap::new(),
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
        self.name_to_ids.clear();
        self.sites.clear();
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
        surface: crate::shared::primitives::LayoutSurface,
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
        self.name_to_ids
            .entry(config.name().clone())
            .or_default()
            .push(id);
        let key = crate::shared::primitives::ModuleKey::new(config.name().clone(), instance_id);
        self.sites.insert(
            id,
            ModuleSiteInfo {
                key: key.clone(),
                parent: parent_id,
                surface,
            },
        );
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
        key: &crate::shared::primitives::ModuleKey,
        options: ModuleOptions,
        surface: crate::shared::primitives::LayoutSurface,
        full_config: &crate::shared::config::domain::Config,
        deps: &crate::features::module_runtime::ports::ModuleRuntimeDependencies<Fact, LS, DS, US>,
    ) -> Result<
        crate::features::module_runtime::ports::SpawnedModule<WatchLayoutSender>,
        crate::features::module_runtime::ports::RegistryLoadError,
    > {
        let cfg = ModuleConfig::new(
            key.name().clone(),
            true,
            crate::shared::config::domain::EngineSelection::Auto,
            options,
        );
        let (id, module) = self.load_single_module(
            &cfg,
            full_config,
            Some(parent_id),
            key.instance_id().cloned(),
            surface,
        )?;

        let (layout_tx, layout_rx) = tokio::sync::watch::channel(HashMap::new());
        let sender = WatchLayoutSender { tx: layout_tx };

        let ctx = self.build_site_context(id, layout_rx, deps);

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

    /// Builds the `ModuleContext` for an existing site's actor from its
    /// durable `ModuleSiteInfo` (`parent` + `surface`) rather than
    /// reconstructing it from scratch — shared by `spawn_all` and
    /// `reload_module` so a hot-reload of a module embedded via
    /// `ui.module()` inside an open popup/panel/tooltip rebuilds onto the
    /// same `LayoutSurface` it was originally spawned on, instead of
    /// silently resetting to `LayoutSurface::Bar` (`ModuleContext::new`'s
    /// default). An id with no recorded site falls back to `Bar` /
    /// no-parent, which only ever happens for a root module during initial
    /// `load()`, before `spawn_all` has a chance to run.
    fn build_site_context<
        Fact: crate::shared::rendering::ports::canvas::CanvasFactory + 'static,
        LS: crate::features::module_runtime::ports::LayoutEventSender + 'static,
        DS: crate::features::layout_engine::domain::DisplayCommandSender + 'static,
        US: crate::features::vdom::domain::UiCommandSender + 'static,
    >(
        &self,
        id: ModuleId,
        layout_rx: tokio::sync::watch::Receiver<
            std::collections::HashMap<
                crate::shared::primitives::MonitorId,
                crate::shared::primitives::ChildBounds,
            >,
        >,
        deps: &crate::features::module_runtime::ports::ModuleRuntimeDependencies<Fact, LS, DS, US>,
    ) -> crate::features::module_runtime::application::ModuleContext<LS, DS, US> {
        let site = self.sites.get(&id);
        let parent_id = site.and_then(ModuleSiteInfo::parent);
        let surface = site.map_or(crate::shared::primitives::LayoutSurface::Bar, |s| s.surface);

        crate::features::module_runtime::application::ModuleContext::new(
            id,
            deps.hub.clone(),
            deps.surface_manager.clone(),
            deps.layout_sender.clone(),
            deps.display_sender.clone(),
            deps.ui_sender.clone(),
            layout_rx,
        )
        .with_parent(parent_id)
        .with_surface(surface)
    }
}

pub struct WatchLayoutSender {
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
> ModuleRegistryPort<Fact, LS, DS, US, WatchLayoutSender> for ModuleRegistry
{
    fn root_module(&self) -> Option<ModuleId> {
        self.root_module
    }

    fn module_ids(&self) -> &[ModuleId] {
        &self.module_ids
    }

    fn module_names(&self) -> HashMap<ModuleId, crate::shared::primitives::ModuleName> {
        self.sites
            .iter()
            .map(|(id, site)| (*id, site.key.name().clone()))
            .collect()
    }

    fn name_to_ids(&self) -> &HashMap<crate::shared::primitives::ModuleName, Vec<ModuleId>> {
        &self.name_to_ids
    }

    fn module_keys(&self) -> HashMap<ModuleId, crate::shared::primitives::ModuleKey> {
        self.sites
            .iter()
            .map(|(id, site)| (*id, site.key.clone()))
            .collect()
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
        self.sites.get(&id).and_then(ModuleSiteInfo::parent)
    }

    fn spawn_module(
        &mut self,
        parent: ModuleId,
        key: &crate::shared::primitives::ModuleKey,
        options: ModuleOptions,
        surface: crate::shared::primitives::LayoutSurface,
        config: &crate::shared::config::domain::Config,
        deps: &crate::features::module_runtime::ports::ModuleRuntimeDependencies<Fact, LS, DS, US>,
    ) -> Result<
        crate::features::module_runtime::ports::SpawnedModule<WatchLayoutSender>,
        crate::features::module_runtime::ports::RegistryLoadError,
    > {
        self.spawn_module_lazy(parent, key, options, surface, config, deps)
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
        let (root_id, root_module) = self.load_single_module(
            &root_cfg,
            config,
            None,
            None,
            crate::shared::primitives::LayoutSurface::Bar,
        )?;
        self.modules.insert(root_id, root_module);
        self.root_module = Some(root_id);

        // 2. Load all configured child modules
        for mod_cfg in config.modules().modules().values() {
            if mod_cfg.is_enabled() && mod_cfg.name() != root_name {
                let (id, module) = self.load_single_module(
                    mod_cfg,
                    config,
                    Some(root_id),
                    None,
                    crate::shared::primitives::LayoutSurface::Bar,
                )?;
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
                                crate::shared::primitives::LayoutSurface::Bar,
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
    ) -> std::collections::HashMap<ModuleId, WatchLayoutSender> {
        let mut layout_senders: std::collections::HashMap<ModuleId, WatchLayoutSender> =
            std::collections::HashMap::new();

        for (id, module) in std::mem::take(&mut self.modules) {
            let (layout_tx, layout_rx) =
                tokio::sync::watch::channel(std::collections::HashMap::new());
            layout_senders.insert(id, WatchLayoutSender { tx: layout_tx });

            let ctx = self.build_site_context(id, layout_rx, deps);

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
        std::collections::HashMap<ModuleId, WatchLayoutSender>,
        crate::features::module_runtime::ports::RegistryLoadError,
    > {
        use crate::app::builtins::BuiltinError;
        use crate::features::module_runtime::ports::RegistryLoadError;

        let mut new_senders: std::collections::HashMap<ModuleId, WatchLayoutSender> =
            std::collections::HashMap::new();
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
            new_senders.insert(id, WatchLayoutSender { tx: layout_tx });

            let ctx = self.build_site_context(id, layout_rx, deps);

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
            WatchLayoutSender,
        >;

    #[test]
    fn test_module_registry_load() {
        let home = crate::test_utils::TestHome::new("registry");
        let mut registry = ModuleRegistry::new(home.env());
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
            .load_single_module(
                &calendar_cfg,
                &config,
                Some(parent_a),
                None,
                crate::shared::primitives::LayoutSurface::Bar,
            )
            .unwrap();
        let (id_b, _) = registry
            .load_single_module(
                &calendar_cfg,
                &config,
                Some(parent_b),
                None,
                crate::shared::primitives::LayoutSurface::Bar,
            )
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

        assert_eq!(
            registry.sites.get(&id_a).and_then(ModuleSiteInfo::parent),
            Some(parent_a)
        );
        assert_eq!(
            registry.sites.get(&id_b).and_then(ModuleSiteInfo::parent),
            Some(parent_b)
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
        let home = crate::test_utils::TestHome::new("registry");
        let mut registry = ModuleRegistry::new(home.env());
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
        let home = crate::test_utils::TestHome::new("registry");
        let mut registry = ModuleRegistry::new(home.env());
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
        let home = crate::test_utils::TestHome::new("registry");
        let mut registry = ModuleRegistry::new(home.env());
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
        let home = crate::test_utils::TestHome::new("registry");
        let mut registry = ModuleRegistry::new(home.env());
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
        let key = crate::shared::primitives::ModuleKey::from_name(name);
        let spawned = registry
            .spawn_module(
                root_id,
                &key,
                crate::shared::primitives::ModuleOptions::default(),
                crate::shared::primitives::LayoutSurface::Popup,
                &config,
                &deps,
            )
            .unwrap();
        let id = spawned.id();

        assert_ne!(id, root_id);
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
        let key = crate::shared::primitives::ModuleKey::from_name(name);
        let result = registry.spawn_module(
            root_id,
            &key,
            crate::shared::primitives::ModuleOptions::default(),
            crate::shared::primitives::LayoutSurface::Bar,
            &config,
            &deps,
        );
        assert!(matches!(
            result,
            Err(crate::features::module_runtime::ports::RegistryLoadError::ModuleNotFound(_))
        ));
    }

    /// Bug A regression: `reload_module` used to rebuild a reloaded
    /// module's `ModuleContext` via `.with_parent(parent_id)` alone, never
    /// chaining `.with_surface(...)` — so hot-reloading a module embedded
    /// via `ui.module()` inside an open popup/panel/tooltip silently reset
    /// its `LayoutSurface` back to the `Bar` default, breaking where its
    /// Wayland subsurface gets parented. `build_site_context` — now shared
    /// by `spawn_all`, `spawn_module_lazy` and `reload_module` — must carry
    /// the site's recorded surface forward across a reload instead.
    #[tokio::test]
    async fn test_reload_preserves_embedded_surface() {
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

        // Spawn "calendar" lazily, embedded under the root as a Popup site
        // — mirroring a module embedded via `ui.module()` inside an open
        // `ui.popup`.
        let name = crate::shared::primitives::ModuleName::new("calendar");
        let key = crate::shared::primitives::ModuleKey::from_name(name.clone());
        let spawned = registry
            .spawn_module(
                root_id,
                &key,
                crate::shared::primitives::ModuleOptions::default(),
                crate::shared::primitives::LayoutSurface::Popup,
                &config,
                &deps,
            )
            .unwrap();
        let id = spawned.id();
        assert_eq!(
            registry.sites.get(&id).map(ModuleSiteInfo::surface),
            Some(crate::shared::primitives::LayoutSurface::Popup)
        );

        // Trigger a reload for that module name — the same path a widget
        // script or config hot-reload drives.
        registry.reload_module(&name, &config, &deps).unwrap();

        // The recorded site facts must survive the reload...
        assert_eq!(
            registry.sites.get(&id).map(ModuleSiteInfo::surface),
            Some(crate::shared::primitives::LayoutSurface::Popup)
        );

        // ...and the ModuleContext `reload_module` actually builds for the
        // rebuilt actor must carry that surface, not reset to
        // `ModuleContext::new`'s `LayoutSurface::Bar` default.
        let (_layout_tx, layout_rx) = tokio::sync::watch::channel(HashMap::new());
        let ctx = registry.build_site_context(id, layout_rx, &deps);
        assert_eq!(
            ctx.surface(),
            crate::shared::primitives::LayoutSurface::Popup
        );
    }
}
