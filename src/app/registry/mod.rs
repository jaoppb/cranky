use crate::shared::config::domain::ModuleConfig;

use crate::features::module_runtime::ports::{AnyModulePort, ModuleRegistryPort};

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
struct ModuleSiteInfo {
    key: crate::shared::primitives::ModuleKey,
    parent: Option<ModuleId>,
    surface: crate::shared::primitives::LayoutSurface,
}

impl ModuleSiteInfo {
    /// The site of a module the configuration declares: keyed by its name
    /// alone, in its parent's bar tree.
    const fn configured(
        name: crate::shared::primitives::ModuleName,
        parent: Option<ModuleId>,
    ) -> Self {
        Self {
            key: crate::shared::primitives::ModuleKey::new(name, None),
            parent,
            surface: crate::shared::primitives::LayoutSurface::Bar,
        }
    }

    #[must_use]
    const fn parent(&self) -> Option<ModuleId> {
        self.parent
    }

    #[cfg(test)]
    #[must_use]
    const fn surface(&self) -> crate::shared::primitives::LayoutSurface {
        self.surface
    }
}

/// A module the registry has loaded and initialised, with the id its site
/// was given, ready for `spawn_site_actor` to run.
struct LoadedModule {
    id: ModuleId,
    module: Box<dyn AnyModulePort>,
}

pub(super) struct ModuleRegistry {
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
    /// Every actor `spawn_module_lazy` has handed off to its own task,
    /// keyed by its `ModuleId`. Unlike a config-declared module's actor
    /// (spawned via `spawn_all`/`reload_module`, which live for the whole
    /// registry generation they belong to), a lazily spawned actor has no
    /// other owner — nothing else calls `clear()`'s `abort()` on it — so
    /// without this it would keep running forever past a config hot-reload,
    /// orphaned and able to collide with a freshly re-minted `ModuleId`
    /// once `next_module_id` resets.
    lazy_actor_handles: HashMap<ModuleId, tokio::task::JoinHandle<()>>,
    dbus_subscriptions: Vec<crate::shared::dbus::domain::DBusSubscription>,
    style_to_modules: HashMap<StyleSheetName, HashSet<crate::shared::primitives::ModuleName>>,
    active_signals: HashSet<crate::shared::events::signals::SignalKind>,
    app_env: std::sync::Arc<crate::shared::env::domain::AppEnvironment>,
}

impl ModuleRegistry {
    #[must_use]
    pub(crate) fn new(app_env: std::sync::Arc<crate::shared::env::domain::AppEnvironment>) -> Self {
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
            lazy_actor_handles: HashMap::new(),
            dbus_subscriptions: Vec::new(),
            style_to_modules: HashMap::new(),
            active_signals: HashSet::new(),
            app_env,
        }
    }

    pub(crate) fn clear(&mut self) {
        // Every lazily spawned actor is orphaned the instant the registry
        // it belongs to is cleared — nothing else will ever abort it, and
        // `next_module_id` resetting below means its old id can be handed
        // to a brand-new actor. Abort first, so that reuse never races a
        // still-running orphan.
        for (_, handle) in self.lazy_actor_handles.drain() {
            handle.abort();
        }

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
    pub(crate) fn modules_using_style(
        &self,
        sheet: &StyleSheetName,
    ) -> Vec<crate::shared::primitives::ModuleName> {
        self.style_to_modules
            .get(sheet)
            .map_or_default(|set| set.iter().cloned().collect())
    }

    #[must_use]
    pub(crate) fn create_style_resolver_for_module(
        &self,
        styles: &[StyleSheetName],
    ) -> std::sync::Arc<dyn StyleResolverPort> {
        let loader = FsStyleLoader::new(self.app_env.clone());
        let parser = LightningCssAdapter::new();
        let mut parsed_sheets: Vec<Box<dyn ParsedStyleSheetPort>> = Vec::new();

        tracing::debug!(requested_styles = ?styles.iter().map(super::super::features::styling::domain::StyleSheetName::as_str).collect::<Vec<_>>(), "Creating composite style resolver for module");

        // 1. Always load base.css first if available
        parsed_sheets.extend(
            StyleSheetName::new("base")
                .ok()
                .and_then(|base_name| load_parsed_sheet(&loader, &parser, &base_name)),
        );

        // 2. Load module stylesheets
        parsed_sheets.extend(
            styles
                .iter()
                .filter(|style_name| style_name.as_str() != "base")
                .filter_map(|style_name| load_parsed_sheet(&loader, &parser, style_name)),
        );

        std::sync::Arc::new(CompositeStyleResolver::new(parsed_sheets))
    }

    /// Mints an id for the module `config` names, finds and initialises it,
    /// and registers its subscriptions, styles and configuration. Where it
    /// sits is recorded apart, by `record_site`.
    fn load_single_module(
        &mut self,
        config: &ModuleConfig,
        full_config: &crate::shared::config::domain::Config,
    ) -> Result<LoadedModule, crate::features::module_runtime::ports::RegistryLoadError> {
        let id = self.mint_module_id();
        let module = self.instantiate_module(config, full_config)?;

        self.record_dbus_subscriptions(module.as_ref());
        self.record_signal_subscriptions(module.as_ref());

        let mod_styles = module.styles();
        log_style_dependencies(config, id, mod_styles);
        self.record_style_dependencies(config.name(), mod_styles);

        self.module_configs.insert(id, config.clone());
        self.name_to_ids
            .entry(config.name().clone())
            .or_default()
            .push(id);

        Ok(LoadedModule { id, module })
    }

    /// Hands out the next `ModuleId`, never reused for the registry's
    /// lifetime (until `clear`).
    const fn mint_module_id(&mut self) -> ModuleId {
        let id = ModuleId::new(self.next_module_id);
        self.next_module_id = self.next_module_id.saturating_add(1);
        id
    }

    /// Finds the script `config` names, for its engine, and initialises it
    /// with `config` and the full configuration.
    fn instantiate_module(
        &self,
        config: &ModuleConfig,
        full_config: &crate::shared::config::domain::Config,
    ) -> Result<Box<dyn AnyModulePort>, crate::features::module_runtime::ports::RegistryLoadError>
    {
        let mut module =
            builtins::BuiltinModules::find_module(config.name(), config.engine(), &self.app_env)
                .map_err(registry_load_error)?;

        module.init(config, full_config).map_err(|error| {
            crate::features::module_runtime::ports::RegistryLoadError::ModuleInit {
                module_name: config.name().clone(),
                source: error,
            }
        })?;
        Ok(module)
    }

    /// Adds the D-Bus subscriptions `module` declares to the ones the
    /// registry registers.
    fn record_dbus_subscriptions(&mut self, module: &dyn AnyModulePort) {
        for sub in module.dbus_subscriptions() {
            self.dbus_subscriptions.push(sub.clone());
        }
    }

    /// Marks the signals `module` subscribes to as active.
    fn record_signal_subscriptions(&mut self, module: &dyn AnyModulePort) {
        for sub in module.subscriptions() {
            self.active_signals.insert(*sub);
        }
    }

    /// Lists the module `name` among the users of each sheet in `styles`.
    fn record_style_dependencies(
        &mut self,
        name: &crate::shared::primitives::ModuleName,
        styles: &[StyleSheetName],
    ) {
        for style_name in styles {
            self.style_to_modules
                .entry(style_name.clone())
                .or_default()
                .insert(name.clone());
        }
    }

    /// Records where the module `id` sits, and lists it among the modules
    /// of the registry.
    fn record_site(&mut self, id: ModuleId, site: ModuleSiteInfo) {
        self.site_index.insert(
            crate::shared::primitives::ModuleSite::new(site.parent, site.key.clone()),
            id,
        );
        self.sites.insert(id, site);
        self.module_ids.push(id);
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
        site: crate::features::module_runtime::ports::LazySite,
        setup: &crate::features::module_runtime::ports::ModuleSetup<'_, Fact, LS, DS, US>,
    ) -> Result<
        crate::features::module_runtime::ports::SpawnedModule<WatchLayoutSender>,
        crate::features::module_runtime::ports::RegistryLoadError,
    > {
        let (parent, surface) = (site.parent(), site.surface());
        let (key, options) = site.into_key_and_options();
        let cfg = ModuleConfig::new(key.name().clone(), options);
        let loaded = self.load_single_module(&cfg, setup.config())?;
        let (id, parent) = (loaded.id, Some(parent));
        self.record_site(
            id,
            ModuleSiteInfo {
                key,
                parent,
                surface,
            },
        );

        let (sender, handle) = self.spawn_site_actor(loaded, setup.deps());
        self.lazy_actor_handles.insert(id, handle);

        Ok(crate::features::module_runtime::ports::SpawnedModule::new(
            id, sender,
        ))
    }

    /// Spawns the actor that runs the loaded module `id` at its recorded
    /// site (see `build_site_context`), rendering with a style resolver
    /// built from the sheets the module declares. Returns the sender its
    /// layout goes through and the actor's task.
    fn spawn_site_actor<
        Fact: crate::shared::rendering::ports::canvas::CanvasFactory + 'static,
        LS: crate::features::module_runtime::ports::LayoutEventSender + 'static,
        DS: crate::features::layout_engine::domain::DisplayCommandSender + 'static,
        US: crate::features::vdom::domain::UiCommandSender + 'static,
    >(
        &self,
        LoadedModule { id, module }: LoadedModule,
        deps: &crate::features::module_runtime::ports::ModuleRuntimeDependencies<Fact, LS, DS, US>,
    ) -> (WatchLayoutSender, tokio::task::JoinHandle<()>) {
        let (ctx, sender) = self.build_site_context(id, deps);

        let style_resolver = self.create_style_resolver_for_module(module.styles());
        let vdom_diff = Arc::new(crate::features::vdom::adapters::DefaultVdomDiffAdapter::new());

        let handle = crate::features::module_runtime::application::ModuleActor::new(
            module,
            ctx,
            crate::features::module_runtime::domain::RenderServices::new(
                deps.canvas_factory.clone(),
                style_resolver,
                vdom_diff,
            ),
        )
        .spawn();
        (sender, handle)
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
    ///
    /// The context reads its layout from a new channel, whose sender comes
    /// back with it for whoever lays the module out.
    fn build_site_context<
        Fact: crate::shared::rendering::ports::canvas::CanvasFactory + 'static,
        LS: crate::features::module_runtime::ports::LayoutEventSender + 'static,
        DS: crate::features::layout_engine::domain::DisplayCommandSender + 'static,
        US: crate::features::vdom::domain::UiCommandSender + 'static,
    >(
        &self,
        id: ModuleId,
        deps: &crate::features::module_runtime::ports::ModuleRuntimeDependencies<Fact, LS, DS, US>,
    ) -> (
        crate::features::module_runtime::application::ModuleContext<LS, DS, US>,
        WatchLayoutSender,
    ) {
        let (layout_tx, layout_rx) = tokio::sync::watch::channel(HashMap::new());
        let site = self.sites.get(&id);
        let parent_id = site.and_then(ModuleSiteInfo::parent);
        let surface = site.map_or(crate::shared::primitives::LayoutSurface::Bar, |site_info| {
            site_info.surface
        });

        let ctx = crate::features::module_runtime::application::ModuleContext::new(
            id,
            deps.handles.clone(),
            layout_rx,
        )
        .with_parent(parent_id)
        .with_surface(surface);
        (ctx, WatchLayoutSender { tx: layout_tx })
    }

    /// Loads the root module, configured under `[modules.<root>]` or else
    /// from the `[root]` options, on the bar with no parent, and makes it
    /// the registry's root.
    fn load_root(
        &mut self,
        config: &crate::shared::config::domain::Config,
    ) -> Result<(), crate::features::module_runtime::ports::RegistryLoadError> {
        let root_name = config.root().name();
        let root_cfg = config.modules().get(root_name).cloned().unwrap_or_else(|| {
            ModuleConfig::new(root_name.clone(), config.root().options().clone())
        });
        let LoadedModule {
            id: root_id,
            module: root_module,
        } = self.load_single_module(&root_cfg, config)?;
        self.record_site(
            root_id,
            ModuleSiteInfo::configured(root_cfg.name().clone(), None),
        );
        self.modules.insert(root_id, root_module);
        self.root_module = Some(root_id);
        Ok(())
    }

    /// Loads every enabled module configured under `[modules]`, except the
    /// root, as a child of the root.
    fn load_configured_children(
        &mut self,
        config: &crate::shared::config::domain::Config,
    ) -> Result<(), crate::features::module_runtime::ports::RegistryLoadError> {
        let root_name = config.root().name();
        let children = config
            .modules()
            .modules()
            .values()
            .filter(|mod_cfg| mod_cfg.is_enabled() && mod_cfg.name() != root_name);
        for mod_cfg in children {
            self.load_child(mod_cfg, config)?;
        }
        Ok(())
    }

    /// Loads, with default options, every module the root's `left`,
    /// `center` and `right` sections name that is not loaded yet, as a
    /// child of the root.
    fn load_section_modules(
        &mut self,
        config: &crate::shared::config::domain::Config,
    ) -> Result<(), crate::features::module_runtime::ports::RegistryLoadError> {
        for mod_name in root_section_module_names(config) {
            self.load_section_module(mod_name, config)?;
        }
        Ok(())
    }

    /// Loads the module `mod_name`, with default options, as a child of the root, unless one by
    /// that name is loaded already.
    fn load_section_module(
        &mut self,
        mod_name: crate::shared::primitives::ModuleName,
        config: &crate::shared::config::domain::Config,
    ) -> Result<(), crate::features::module_runtime::ports::RegistryLoadError> {
        if self.name_to_ids.contains_key(&mod_name) {
            return Ok(());
        }
        let auto_cfg = ModuleConfig::new(mod_name, ModuleOptions::default());
        self.load_child(&auto_cfg, config)
    }

    /// Loads the module `cfg` names on the bar under the root `load_root`
    /// set, into the pool `spawn_all` spawns.
    fn load_child(
        &mut self,
        cfg: &ModuleConfig,
        config: &crate::shared::config::domain::Config,
    ) -> Result<(), crate::features::module_runtime::ports::RegistryLoadError> {
        let LoadedModule { id, module } = self.load_single_module(cfg, config)?;
        self.record_site(
            id,
            ModuleSiteInfo::configured(cfg.name().clone(), self.root_module),
        );
        self.modules.insert(id, module);
        Ok(())
    }

    /// Every site whose module is configured under `name`, with that
    /// configuration.
    fn configs_named(
        &self,
        name: &crate::shared::primitives::ModuleName,
    ) -> Vec<(ModuleId, ModuleConfig)> {
        self.module_configs
            .iter()
            .filter(|(_, cfg)| cfg.name() == name)
            .map(|(id, cfg)| (*id, cfg.clone()))
            .collect()
    }
}

/// The sheet `name`, read by `loader` and parsed by `parser`, or `None` if
/// either step fails.
fn load_parsed_sheet(
    loader: &FsStyleLoader,
    parser: &LightningCssAdapter,
    name: &StyleSheetName,
) -> Option<Box<dyn ParsedStyleSheetPort>> {
    let css = loader.load_stylesheet(name).ok()?;
    parser.parse_stylesheet(name.clone(), &css).ok()
}

/// The registry error a failed script lookup stands for.
fn registry_load_error(
    error: crate::app::builtins::BuiltinError,
) -> crate::features::module_runtime::ports::RegistryLoadError {
    use crate::app::builtins::BuiltinError;
    use crate::features::module_runtime::ports::RegistryLoadError;

    match error {
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
        BuiltinError::Env(message) | BuiltinError::InputOutput(message) => {
            RegistryLoadError::Internal(message)
        }
    }
}

/// Logs the sheets the module `config` names depends on, under its new `id`.
///
/// Kept out of line to keep its caller under `cognitive_complexity`.
fn log_style_dependencies(config: &ModuleConfig, id: ModuleId, mod_styles: &[StyleSheetName]) {
    tracing::debug!(
        module = %config.name().as_str(),
        id = %id,
        styles = ?mod_styles.iter().map(super::super::features::styling::domain::StyleSheetName::as_str).collect::<Vec<_>>(),
        "Registered module style dependencies"
    );
}

/// The module names, in order, listed in the root's `left`, `center` and
/// `right` sections, skipping a section that is not a list and an item
/// that is not a string.
fn root_section_module_names(
    config: &crate::shared::config::domain::Config,
) -> impl Iterator<Item = crate::shared::primitives::ModuleName> + '_ {
    let check_sections = ["left", "center", "right"];
    check_sections
        .into_iter()
        .filter_map(|sec| match config.root().options().get(sec) {
            Some(DynamicValue::Array(arr)) => Some(arr),
            _ => None,
        })
        .flatten()
        .filter_map(|item| {
            item.as_str()
                .map(crate::shared::primitives::ModuleName::new)
        })
}

pub(super) struct WatchLayoutSender {
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
    pub(crate) fn new(
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
        site: crate::features::module_runtime::ports::LazySite,
        setup: crate::features::module_runtime::ports::ModuleSetup<'_, Fact, LS, DS, US>,
    ) -> Result<
        crate::features::module_runtime::ports::SpawnedModule<WatchLayoutSender>,
        crate::features::module_runtime::ports::RegistryLoadError,
    > {
        self.spawn_module_lazy(site, &setup)
    }

    fn load(
        &mut self,
        config: &crate::shared::config::domain::Config,
    ) -> Result<(), crate::features::module_runtime::ports::RegistryLoadError> {
        self.clear();

        // 1. Load the root module (default "bar")
        self.load_root(config)?;
        // 2. Load all configured child modules
        self.load_configured_children(config)?;
        // 3. Also load any modules referenced in root options (left, center, right)
        self.load_section_modules(config)?;

        Ok(())
    }

    fn spawn_all(
        &mut self,
        deps: &crate::features::module_runtime::ports::ModuleRuntimeDependencies<Fact, LS, DS, US>,
    ) -> std::collections::HashMap<ModuleId, WatchLayoutSender> {
        let mut layout_senders: std::collections::HashMap<ModuleId, WatchLayoutSender> =
            std::collections::HashMap::new();

        for (id, module) in std::mem::take(&mut self.modules) {
            let (sender, _) = self.spawn_site_actor(LoadedModule { id, module }, deps);
            layout_senders.insert(id, sender);
        }

        layout_senders
    }

    fn reload_module(
        &mut self,
        name: &crate::shared::primitives::ModuleName,
        setup: crate::features::module_runtime::ports::ModuleSetup<'_, Fact, LS, DS, US>,
    ) -> Result<
        std::collections::HashMap<ModuleId, WatchLayoutSender>,
        crate::features::module_runtime::ports::RegistryLoadError,
    > {
        let (config, deps) = (setup.config(), setup.deps());

        let mut new_senders: std::collections::HashMap<ModuleId, WatchLayoutSender> =
            std::collections::HashMap::new();

        for (id, cfg) in self.configs_named(name) {
            let module = self.instantiate_module(&cfg, config)?;
            self.record_style_dependencies(cfg.name(), module.styles());
            self.record_signal_subscriptions(module.as_ref());

            let (sender, _) = self.spawn_site_actor(LoadedModule { id, module }, deps);
            new_senders.insert(id, sender);
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
            if let Err(error) = dbus.subscribe(sub.clone()).await {
                tracing::error!("Failed to subscribe to DBus: {error}");
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
mod tests;
