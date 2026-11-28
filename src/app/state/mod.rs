//! The application actor: `CrankyApp` owns the module registry and the read model, and routes
//! display, layout, UI and system commands between the modules and the Wayland adapter.

use crate::app::commands::SystemCommand;
use crate::features::layout_engine::domain::{DisplayCommand, FloatingKind, ShowFloating};
use crate::features::module_runtime::ports::{
    LayoutEvent, LazySite, ModuleRuntimeDependencies, ModuleSetup, SpawnedModule,
};
use crate::features::vdom::domain::UiCommand;
use crate::shared::config::domain::Config;
use crate::shared::events::signals::{HyprlandState, SignalHub};
use crate::shared::primitives::{
    ChildBounds, ChildModuleLayout, ChildReport, ModuleId, ModuleSite, MonitorId,
};
use crate::shared::wayland::ports::{DisplayServerError, DisplayServerPort};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;

mod run_loop;

use run_loop::{LoopPorts, SignalReceivers};

/// Why the application could not start its modules or keep its loop running.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    /// Loading the modules the configuration names failed.
    #[error("Module error: {0}")]
    Module(#[from] crate::features::module_runtime::ports::RegistryLoadError),
    /// Waiting for or dispatching display server events failed.
    #[error("Internal error: {message}")]
    Internal {
        /// The display server error, as text.
        message: String,
    },
}

pub use crate::shared::wayland::domain::AppReadModel;

/// The module and monitor a layout event is about: the module that reported a size or laid out
/// its children, and the monitor it did so on.
#[derive(Clone, Copy, Debug)]
struct ModuleOnMonitor<'a> {
    module: ModuleId,
    monitor: &'a MonitorId,
}

impl<'a> ModuleOnMonitor<'a> {
    const fn new(module: ModuleId, monitor: &'a MonitorId) -> Self {
        Self { module, monitor }
    }
}

pub(super) struct CrankyApp<
    R: crate::features::module_runtime::ports::ModuleRegistryPort<F, LS, DS, US, Snd> + 'static,
    F: crate::shared::rendering::ports::canvas::CanvasFactory + 'static,
    LS: crate::features::module_runtime::ports::LayoutEventSender + 'static,
    DS: crate::features::layout_engine::domain::DisplayCommandSender + 'static,
    US: crate::features::vdom::domain::UiCommandSender + 'static,
    Snd: crate::features::module_runtime::ports::LayoutSender + 'static,
> {
    /// The hub, the surface manager, the outbound channels and the canvas
    /// factory every module actor is spawned with; the app reads the hub too.
    deps: crate::features::module_runtime::ports::ModuleRuntimeDependencies<F, LS, DS, US>,
    read_model: AppReadModel,
    display_rx: mpsc::Receiver<DisplayCommand>,
    layout_rx: mpsc::Receiver<LayoutEvent>,
    ui_rx: mpsc::Receiver<UiCommand>,
    system_rx: mpsc::Receiver<SystemCommand>,
    layout_senders: HashMap<ModuleId, Snd>,
    registry: Box<R>,
    /// The monitor Hyprland last reported focused, empty until it reports one.
    focused_monitor: String,
    /// `None` until `set_adapter_supervisor` is called — `bootstrap` wires it
    /// in once `init_secondary_adapters` has opened the `DBus` connections,
    /// which happens after `CrankyApp::new`. A lazy spawn before that point simply
    /// can't ensure any adapter; nothing in v1 lazily spawns before boot
    /// finishes, so this is never actually observed empty in practice.
    adapter_supervisor: Option<crate::app::adapter_supervisor::AdapterSupervisor>,
}

/// The receiving ends of the three channels module actors send through: the
/// ends of the senders in `ModuleSenders`.
pub(super) struct ModuleReceivers {
    layout: mpsc::Receiver<LayoutEvent>,
    display: mpsc::Receiver<DisplayCommand>,
    interface: mpsc::Receiver<UiCommand>,
}

impl ModuleReceivers {
    #[must_use]
    pub(crate) const fn new(
        layout: mpsc::Receiver<LayoutEvent>,
        display: mpsc::Receiver<DisplayCommand>,
        interface: mpsc::Receiver<UiCommand>,
    ) -> Self {
        Self {
            layout,
            display,
            interface,
        }
    }
}

/// The session-bus adapters the app loop drives: the `DBus` signal
/// subscriptions the modules asked for, registered when the loop starts, and
/// the systray host that UI commands trigger item actions on.
pub(super) struct BusAdapters<S> {
    subscriptions: crate::shared::dbus::subscription_manager::DbusSubscriptionManager,
    systray: S,
}

impl<S> BusAdapters<S> {
    #[must_use]
    pub(crate) const fn new(
        subscriptions: crate::shared::dbus::subscription_manager::DbusSubscriptionManager,
        systray: S,
    ) -> Self {
        Self {
            subscriptions,
            systray,
        }
    }
}

pub(super) struct StateChannels {
    modules: ModuleReceivers,
    system: mpsc::Receiver<SystemCommand>,
}

impl StateChannels {
    #[must_use]
    pub(crate) const fn new(
        modules: ModuleReceivers,
        system: mpsc::Receiver<SystemCommand>,
    ) -> Self {
        Self { modules, system }
    }
}

impl<
    R: crate::features::module_runtime::ports::ModuleRegistryPort<F, LS, DS, US, Snd> + 'static,
    F: crate::shared::rendering::ports::canvas::CanvasFactory + 'static,
    LS: crate::features::module_runtime::ports::LayoutEventSender + 'static,
    DS: crate::features::layout_engine::domain::DisplayCommandSender + 'static,
    US: crate::features::vdom::domain::UiCommandSender + 'static,
    Snd: crate::features::module_runtime::ports::LayoutSender + 'static,
> CrankyApp<R, F, LS, DS, US, Snd>
{
    /// Creates a new [`CrankyApp`] instance and initializes modules from the
    /// configuration the hub holds, which at boot is the initial one.
    ///
    /// # Errors
    ///
    /// Returns [`AppError::Module`] if initial module loading fails.
    pub(crate) fn new(
        deps: ModuleRuntimeDependencies<F, LS, DS, US>,
        channels: StateChannels,
        mut registry: Box<R>,
    ) -> Result<Self, AppError> {
        let config = deps.handles.hub().config_rx().borrow().clone();
        registry.load(&config).map_err(AppError::Module)?;
        let read_model = Self::read_model_of(&registry, config);

        let mut app = Self {
            deps,
            read_model,
            display_rx: channels.modules.display,
            layout_rx: channels.modules.layout,
            ui_rx: channels.modules.interface,
            system_rx: channels.system,
            layout_senders: HashMap::new(),
            registry,
            focused_monitor: String::new(),
            adapter_supervisor: None,
        };
        app.layout_senders = app.registry.spawn_all(&app.deps);
        Ok(app)
    }

    /// The read model of the modules `registry` just loaded from `config`, with no size reported
    /// and no layout computed yet.
    fn read_model_of(registry: &R, config: Config) -> AppReadModel {
        AppReadModel {
            config,
            root_module: registry.root_module(),
            module_ids: registry.module_ids().to_vec(),
            module_names: registry.module_names(),
            name_to_ids: registry.name_to_ids().clone(),
            module_keys: registry.module_keys(),
            module_sizes: HashMap::new(),
            computed_layouts: HashMap::new(),
        }
    }

    const fn hub(&self) -> &Arc<SignalHub> {
        self.deps.handles.hub()
    }

    /// Wires in the on-demand adapter starter (decision 3). Called from
    /// `main.rs` once the `DBus` connection it needs is established — after
    /// `CrankyApp::new`, since that connection failing is fatal at boot and
    /// must not be entangled with module loading.
    pub(crate) fn set_adapter_supervisor(
        &mut self,
        supervisor: crate::app::adapter_supervisor::AdapterSupervisor,
    ) {
        self.adapter_supervisor = Some(supervisor);
    }

    #[must_use]
    pub(crate) fn active_signals(
        &self,
    ) -> &std::collections::HashSet<crate::shared::events::signals::SignalKind> {
        self.registry.active_signal_subscriptions()
    }

    /// Carries out `initial` and every display command queued behind it, then renders once if
    /// any of them asked for it.
    fn handle_display_commands(
        &mut self,
        initial: DisplayCommand,
        display: &mut impl DisplayServerPort,
    ) {
        let mut needs_render = false;
        let mut next = Some(initial);
        while let Some(cmd) = next {
            needs_render |= Self::apply_display_command(cmd, display);
            next = self.display_rx.try_recv().ok();
        }
        if needs_render {
            let _ = display.render_all(&self.read_model, &self.layout_senders);
        }
    }

    /// Carries out one display command, and says whether it asks for a render.
    fn apply_display_command(cmd: DisplayCommand, display: &mut impl DisplayServerPort) -> bool {
        match cmd {
            DisplayCommand::RequestRender => return true,
            DisplayCommand::ShowFloatingSurface(floating) => {
                Self::show_floating_surface(floating, display);
            }
            DisplayCommand::HideFloatingSurface { kind } => {
                Self::hide_floating_surface(&kind, display);
            }
        }
        false
    }

    /// Asks the display to show `floating`, logging the request and its outcome.
    fn show_floating_surface(floating: ShowFloating, display: &mut impl DisplayServerPort) {
        log_show_requested(&floating);
        if let Err(error) = display.show_floating_surface(floating) {
            log_show_failed(&error);
        } else {
            log_show_succeeded();
        }
    }

    /// Asks the display to hide the floating surface `kind`, logging the request and its outcome.
    fn hide_floating_surface(kind: &FloatingKind, display: &mut impl DisplayServerPort) {
        log_hide_requested(kind);
        if let Err(error) = display.hide_floating_surface(kind) {
            log_hide_failed(&error);
        } else {
            log_hide_succeeded();
        }
    }

    /// Records the child layouts `parent` computed on its monitor: a child whose site resolves is
    /// placed there, one that does not is lazily spawned, and a child the parent no longer lays
    /// out on that monitor is suspended there.
    async fn handle_container_layouts(
        &mut self,
        parent: ModuleOnMonitor<'_>,
        layouts: &[ChildModuleLayout],
    ) {
        let mut seen = std::collections::HashSet::new();
        for child_layout in layouts {
            seen.extend(self.place_or_spawn_child(parent, child_layout).await);
        }

        self.suspend_stale_children(parent, &seen);
    }

    /// Places the child `child_layout` describes when its site resolves, returning its id, and
    /// lazily spawns it when it does not.
    async fn place_or_spawn_child(
        &mut self,
        parent: ModuleOnMonitor<'_>,
        child_layout: &ChildModuleLayout,
    ) -> Option<ModuleId> {
        let resolved = self
            .registry
            .resolve_site(Some(parent.module), child_layout.key());
        let Some(child_id) = resolved else {
            self.ensure_lazy_spawn(Self::lazy_site(parent.module, child_layout))
                .await;
            return None;
        };
        let child = ModuleOnMonitor::new(child_id, parent.monitor);
        self.place_child(child, child_layout.bounds());
        Some(child_id)
    }

    /// Records that `child` is laid out at `bounds` on its monitor, and sends the child its
    /// bounds on every monitor.
    fn place_child(&mut self, child: ModuleOnMonitor<'_>, bounds: ChildBounds) {
        let ModuleOnMonitor {
            module: child_id,
            monitor: monitor_id,
        } = child;
        self.read_model
            .computed_layouts
            .entry(monitor_id.clone())
            .or_default()
            .insert(child_id, bounds);

        tracing::trace!(
            child = %child_id,
            monitor = %monitor_id,
            bounds = ?bounds.rect(),
            "Updating computed_layouts for child module"
        );
        self.resend_child_bounds(child_id);
    }

    /// The site to lazily spawn for a child `parent_id` laid out at a key no module resolves:
    /// the key, options and surface of `child_layout`.
    fn lazy_site(parent_id: ModuleId, child_layout: &ChildModuleLayout) -> LazySite {
        LazySite::new(parent_id, child_layout.key().clone())
            .with_options(child_layout.options().clone())
            .with_surface(child_layout.surface())
    }

    /// Sends `child_id` its full, current cross-monitor bounds map, read back
    /// from `computed_layouts` rather than passed in — the same map both
    /// `handle_container_layouts` (a child gained or moved on a monitor) and
    /// `suspend_stale_children` (a child lost a monitor) need to resend.
    fn resend_child_bounds(&self, child_id: ModuleId) {
        let Some(sender) = self.layout_senders.get(&child_id) else {
            return;
        };
        let child_monitors = self
            .read_model
            .computed_layouts
            .iter()
            .filter_map(|(mon, mod_map)| {
                mod_map.get(&child_id).map(|&bounds| (mon.clone(), bounds))
            })
            .collect();
        sender.send_layout(child_monitors);
    }

    /// Any child of `parent_id` that had a rect on `monitor_id` before this
    /// render but isn't in `seen` now has had its embedding site withdrawn —
    /// its popup/panel/tooltip closed, or the parent stopped embedding it.
    /// Clearing the stale entry and resending the (now smaller) bounds map
    /// is what actually suspends it (decision 14): once `monitor_id` is gone
    /// from the child's own `layout_rx`, its `discover_monitors` excludes
    /// the monitor entirely — no refresh, no render, no diff, no
    /// `submit_buffer` — because it's already in that actor's `render_trees`
    /// (seen before) but no longer has a live rect there.
    fn suspend_stale_children(
        &mut self,
        parent: ModuleOnMonitor<'_>,
        seen: &std::collections::HashSet<ModuleId>,
    ) {
        for child_id in self.stale_children(parent, seen) {
            self.withdraw_child(child_id, parent.monitor);
        }
    }

    /// The children of `parent` with a rect on its monitor that are not in `seen`.
    fn stale_children(
        &self,
        parent: ModuleOnMonitor<'_>,
        seen: &std::collections::HashSet<ModuleId>,
    ) -> Vec<ModuleId> {
        let Some(mod_map) = self.read_model.computed_layouts.get(parent.monitor) else {
            return Vec::new();
        };
        mod_map
            .keys()
            .copied()
            .filter(|id| !seen.contains(id) && self.registry.parent_of(*id) == Some(parent.module))
            .collect()
    }

    /// Clears the rect of `child_id` on `monitor_id`, and sends the child its remaining bounds.
    fn withdraw_child(&mut self, child_id: ModuleId, monitor_id: &MonitorId) {
        if let Some(mod_map) = self.read_model.computed_layouts.get_mut(monitor_id) {
            mod_map.remove(&child_id);
        }
        log_child_withdrawn(child_id, monitor_id);
        self.resend_child_bounds(child_id);
    }

    /// Rendering a `Module` node with no resolved site is the lazy-discovery
    /// signal (decision 2): spawn it here, on the spot, rather than waiting
    /// for a config stanza that will never come. Idempotent two ways — a
    /// site that already resolves is never re-spawned, and a site with a
    /// previously recorded terminal failure is never retried until a hot
    /// reload clears it — so a popup left open behind a ticking parent
    /// doesn't retry a typo'd module name every frame.
    async fn ensure_lazy_spawn(&mut self, site: LazySite) {
        let parent = site.parent();
        let module_site = ModuleSite::new(Some(parent), site.key().clone());
        if self
            .hub()
            .module_errors_rx()
            .borrow()
            .contains_key(&module_site)
        {
            return;
        }

        if self.would_cycle(parent, module_site.key()) {
            self.record_module_error(
                module_site,
                "would create a cycle with an ancestor".to_string(),
            );
            return;
        }

        self.spawn_lazily(site, module_site).await;
    }

    /// Asks the registry to spawn `site`, whose parent and key `module_site` names, and records
    /// the failure against `module_site` when it cannot.
    async fn spawn_lazily(&mut self, site: LazySite, module_site: ModuleSite) {
        let parent = site.parent();
        let setup = ModuleSetup::new(&self.read_model.config, &self.deps);
        match self.registry.spawn_module(site, setup) {
            Ok(spawned) => {
                log_lazily_spawned(module_site.key(), parent, spawned.id());
                self.adopt_lazy_module(spawned).await;
            }
            Err(error) => {
                log_lazy_spawn_failed(module_site.key(), parent, &error);
                self.record_module_error(module_site, error.to_string());
            }
        }
    }

    /// Takes in a lazily spawned module: keeps its layout sender, refreshes the module lists of
    /// the read model, and starts the adapters the modules now subscribe to.
    async fn adopt_lazy_module(&mut self, spawned: SpawnedModule<Snd>) {
        let id = spawned.id();
        self.layout_senders.insert(id, spawned.into_sender());
        self.refresh_module_lists();
        self.ensure_active_adapters().await;
    }

    /// Copies the registry's module ids, names, ids by name and keys into the read model.
    fn refresh_module_lists(&mut self) {
        self.read_model.module_ids = self.registry.module_ids().to_vec();
        self.read_model.module_names = self.registry.module_names();
        self.read_model
            .name_to_ids
            .clone_from(self.registry.name_to_ids());
        self.read_model.module_keys = self.registry.module_keys();
    }

    /// Starts, through the adapter supervisor once it is wired in, the adapter behind every
    /// signal a loaded module subscribes to.
    async fn ensure_active_adapters(&mut self) {
        let Some(supervisor) = self.adapter_supervisor.as_mut() else {
            return;
        };
        let kinds: Vec<_> = self
            .registry
            .active_signal_subscriptions()
            .iter()
            .copied()
            .collect();
        for kind in kinds {
            supervisor.ensure(kind).await;
        }
    }

    /// Walks the ancestor chain comparing each ancestor's full `ModuleKey`
    /// (name + instance id) against `key`, not just its name — two sites
    /// embedding the same module name with distinct `instance_id`s (e.g.
    /// two `ui.module("clock", {instance_id=...})` calls at different
    /// popup nesting depths) are different sites and must not be rejected
    /// as a cycle against each other. Mirrors how popup-nesting cycle
    /// detection elsewhere (`popup::chain::ancestors`) compares full target
    /// identity rather than bare name.
    fn would_cycle(&self, parent: ModuleId, key: &crate::shared::primitives::ModuleKey) -> bool {
        let module_keys = self.registry.module_keys();
        std::iter::successors(Some(parent), |&id| self.registry.parent_of(id))
            .any(|id| module_keys.get(&id) == Some(key))
    }

    fn record_module_error(&self, site: crate::shared::primitives::ModuleSite, reason: String) {
        let mut errors = self.hub().module_errors_rx().borrow().clone();
        errors.insert(site, reason);
        let _ = self.hub().module_errors_tx().send(errors);
    }

    /// Records `initial` and every layout event queued behind it, then renders.
    async fn handle_layout_events(
        &mut self,
        initial: LayoutEvent,
        display: &mut impl DisplayServerPort,
    ) {
        let mut next = Some(initial);
        while let Some(event) = next {
            self.apply_layout_event(event).await;
            next = self.layout_rx.try_recv().ok();
        }
        let _ = display.render_all(&self.read_model, &self.layout_senders);
    }

    /// Records one layout event: the child layouts a container computed, or a module's new size.
    async fn apply_layout_event(&mut self, event: LayoutEvent) {
        match event {
            LayoutEvent::ContainerLayoutsCalculated {
                parent_id,
                monitor_id,
                layouts,
            } => {
                let parent = ModuleOnMonitor::new(parent_id, &monitor_id);
                self.handle_container_layouts(parent, &layouts).await;
            }
            LayoutEvent::ModuleSizeChanged {
                monitor_id,
                module_id,
                size,
                settled_width,
            } => {
                let module = ModuleOnMonitor::new(module_id, &monitor_id);
                self.handle_size_changed(module, ChildReport::new(size, settled_width));
            }
        }
    }

    /// Carries out `initial` and every UI command queued behind it, in order.
    async fn handle_ui_commands(
        &mut self,
        initial: UiCommand,
        sni: &impl crate::features::systray::ports::SniPort,
    ) {
        let mut next = Some(initial);
        while let Some(cmd) = next {
            Self::apply_ui_command(cmd, sni).await;
            next = self.ui_rx.try_recv().ok();
        }
    }

    /// Carries out one UI command: a shell command, or a systray item action.
    async fn apply_ui_command(cmd: UiCommand, sni: &impl crate::features::systray::ports::SniPort) {
        match cmd {
            UiCommand::Exec(cmd_str) => Self::exec_in_shell(cmd_str),
            UiCommand::SystrayAction { id, action, pos } => {
                log_systray_requested(&id, &action, pos);
                let call = crate::features::systray::domain::SystrayActionCall::new(action, pos);
                Self::trigger_systray_action(sni, &id, &call).await;
            }
        }
    }

    /// Starts `sh -c cmd_str` without waiting for it.
    fn exec_in_shell(cmd_str: String) {
        tracing::debug!("Executing shell command: {cmd_str}");
        let _ = std::process::Command::new("sh")
            .arg("-c")
            .arg(cmd_str)
            .spawn();
    }

    /// Asks the systray host to make `call` on the item `id`, logging the outcome.
    async fn trigger_systray_action(
        sni: &impl crate::features::systray::ports::SniPort,
        id: &crate::features::systray::domain::SystrayId,
        call: &crate::features::systray::domain::SystrayActionCall,
    ) {
        match sni.trigger_action(id, call).await {
            Ok(()) => log_systray_succeeded(id, call.name()),
            Err(error) => log_systray_failed(id, call.name(), &error),
        }
    }

    /// Reloads the modules a changed stylesheet styles — every module for the base sheet — and
    /// says whether a render is due.
    fn handle_reload_style(
        &mut self,
        sheet_name: &crate::features::styling::domain::StyleSheetName,
        deps: &ModuleRuntimeDependencies<F, LS, DS, US>,
    ) -> bool {
        log_reloading_style(sheet_name);
        if sheet_name.as_str() == "base" {
            self.reload_every_module(deps);
            true
        } else {
            self.reload_modules_using(sheet_name, deps)
        }
    }

    /// Reloads every loaded module, keeping the senders of those that reloaded; failures are not
    /// logged.
    fn reload_every_module(&mut self, deps: &ModuleRuntimeDependencies<F, LS, DS, US>) {
        log_base_sheet_changed();
        let all_modules: Vec<_> = self.read_model.module_names.values().cloned().collect();
        for mod_name in all_modules {
            let _ = self.reload_module(&mod_name, deps);
        }
    }

    /// Reloads the modules that use `sheet_name`, and says whether any of them reloaded.
    fn reload_modules_using(
        &mut self,
        sheet_name: &crate::features::styling::domain::StyleSheetName,
        deps: &ModuleRuntimeDependencies<F, LS, DS, US>,
    ) -> bool {
        let mods = self.registry.modules_using_style(sheet_name);
        log_reloading_dependents(sheet_name, &mods);
        let mut any_reloaded = false;
        for mod_name in mods {
            any_reloaded |= self.reload_or_log(&mod_name, deps);
        }
        any_reloaded
    }

    /// Reloads the module called `name`, and says whether it reloaded; a failure is logged.
    fn reload_or_log(
        &mut self,
        name: &crate::shared::primitives::ModuleName,
        deps: &ModuleRuntimeDependencies<F, LS, DS, US>,
    ) -> bool {
        match self.reload_module(name, deps) {
            Ok(()) => true,
            Err(error) => {
                log_reload_failed(name, &error);
                false
            }
        }
    }

    /// Reloads the module called `name` under the current configuration, and keeps the layout
    /// senders of its new actors.
    fn reload_module(
        &mut self,
        name: &crate::shared::primitives::ModuleName,
        deps: &ModuleRuntimeDependencies<F, LS, DS, US>,
    ) -> Result<(), crate::features::module_runtime::ports::RegistryLoadError> {
        let setup = ModuleSetup::new(&self.read_model.config, deps);
        let new_senders = self.registry.reload_module(name, setup)?;
        for (id, sender) in new_senders {
            self.layout_senders.insert(id, sender);
        }
        Ok(())
    }

    /// Carries out `initial` and every system command queued behind it, then renders once if
    /// any of them reloaded a module.
    fn handle_system_commands(
        &mut self,
        initial: SystemCommand,
        display: &mut impl DisplayServerPort,
    ) {
        let mut needs_render = false;
        let mut next = Some(initial);
        let deps = self.deps.clone();
        while let Some(cmd) = next {
            needs_render |= self.apply_system_command(cmd, &deps);
            next = self.system_rx.try_recv().ok();
        }
        if needs_render {
            let _ = display.render_all(&self.read_model, &self.layout_senders);
        }
    }

    /// Carries out one system command, and says whether a render is due.
    fn apply_system_command(
        &mut self,
        cmd: SystemCommand,
        deps: &ModuleRuntimeDependencies<F, LS, DS, US>,
    ) -> bool {
        match cmd {
            SystemCommand::ReloadModule(name) => {
                log_reloading_module(&name);
                self.reload_or_log(&name, deps)
            }
            SystemCommand::ReloadStyle(sheet_name) => self.handle_reload_style(&sheet_name, deps),
        }
    }

    fn handle_config_changed(&mut self, new_config: Config) {
        log_config_reloading();
        self.read_model.config = new_config;
        self.read_model.module_sizes.clear();

        self.registry.clear();
        if let Err(error) = self.registry.load(&self.read_model.config) {
            log_registry_reload_failed(&error);
        } else {
            self.adopt_reloaded_registry();
        }
    }

    /// Takes the registry's root module and module lists into the read model after it reloaded,
    /// and spawns every module again.
    fn adopt_reloaded_registry(&mut self) {
        self.read_model.root_module = self.registry.root_module();
        self.refresh_module_lists();
        self.layout_senders = self.registry.spawn_all(&self.deps);
    }

    fn handle_hyprland_changed(
        &mut self,
        state: &HyprlandState,
        display: &mut impl DisplayServerPort,
    ) {
        let new_focused = state
            .focused_monitor()
            .map_or_default(|monitor| monitor.as_str().to_string());

        if new_focused != self.focused_monitor {
            self.focused_monitor = new_focused;
            let _ = display.render_all(&self.read_model, &self.layout_senders);
        }
    }

    /// Drops every per-monitor entry for a monitor Wayland no longer
    /// reports. Wayland is authoritative for "which monitors exist" (see
    /// `discover_monitors`) — this runs off `monitor_scales` changing,
    /// not Hyprland, since Hyprland's own view can lag a hotplug.
    ///
    /// `module_sizes` / `computed_layouts` (in `read_model`) and the shared
    /// `module_sizes` signal only ever gain entries elsewhere — nothing
    /// removes one when a monitor disconnects. Left alone, a disconnected
    /// monitor stays "discovered" forever, which is what kept
    /// `workspace.lua` rendering — and re-triggering the empty-container
    /// crash path — for a monitor with no surface left.
    fn prune_removed_monitors(&mut self) {
        let live: std::collections::HashSet<MonitorId> = self
            .hub()
            .monitor_scales_rx()
            .borrow()
            .keys()
            .cloned()
            .collect();

        self.read_model
            .module_sizes
            .retain(|id, _| live.contains(id));
        self.read_model
            .computed_layouts
            .retain(|id, _| live.contains(id));

        let mut sizes_map = self.hub().module_sizes_rx().borrow().clone();
        let before = sizes_map.len();
        sizes_map.retain(|id, _| live.contains(id));
        if sizes_map.len() != before {
            let _ = self.hub().module_sizes_tx().send(sizes_map);
        }
    }

    /// Runs the main event loop, listening for display events, commands, and signals.
    ///
    /// # Errors
    ///
    /// Returns [`AppError::Internal`] if display server communication or event dispatching fails.
    pub(crate) async fn run(
        &mut self,
        display: impl DisplayServerPort,
        bus: BusAdapters<impl crate::features::systray::ports::SniPort>,
    ) -> Result<(), AppError> {
        let BusAdapters {
            mut subscriptions,
            systray: sni,
        } = bus;
        let mut signals = SignalReceivers::new(self.hub());

        self.registry
            .register_dbus_subscriptions(&mut subscriptions)
            .await;

        let mut ports = LoopPorts { display, sni };
        loop {
            let _ = ports.display.flush();
            let wakeup = self.next_wakeup(&mut ports.display, &mut signals).await;
            self.handle_wakeup(wakeup, &mut ports).await?;
        }
    }

    fn handle_size_changed(&mut self, module: ModuleOnMonitor<'_>, report: ChildReport) {
        let ModuleOnMonitor {
            module: module_id,
            monitor: monitor_id,
        } = module;
        let size = *report.size();
        let key = self.read_model.module_keys.get(&module_id);
        tracing::trace!(monitor = %monitor_id, module = %module_id, ?size, ?key, "handle_size_changed called");
        self.read_model
            .module_sizes
            .entry(monitor_id.clone())
            .or_default()
            .insert(module_id, size);

        self.report_size_on_hub(module, report);
    }

    /// Publishes on the hub the size `report` of `module` on its monitor, under the module's key;
    /// a module with no key is not published.
    fn report_size_on_hub(&self, module: ModuleOnMonitor<'_>, report: ChildReport) {
        let Some(key) = self.read_model.module_keys.get(&module.module).cloned() else {
            return;
        };
        let monitor_id = module.monitor;
        let mut sizes_map = self.hub().module_sizes_rx().borrow().clone();
        let mon_entry = sizes_map.entry(monitor_id.clone()).or_default();
        mon_entry.insert_report(key, report);
        let _ = self.hub().module_sizes_tx().send(sizes_map);
    }
}

// Each tracing macro adds to the cognitive complexity of the function that calls it, so the
// handlers log through these wrappers. They stay in this file so that every event keeps the
// `cranky::app::state` target.

/// Logs that a floating surface is about to be shown, with where and how.
fn log_show_requested(floating: &ShowFloating) {
    tracing::debug!(
        kind = ?floating.kind,
        monitor_id = ?floating.monitor_id,
        anchor_rect = ?floating.anchor_rect,
        offset = ?floating.offset,
        parent = ?floating.parent,
        "Received DisplayCommand::ShowFloatingSurface, calling display.show_floating_surface"
    );
}

/// Logs that the display refused to show a floating surface.
fn log_show_failed(error: &DisplayServerError) {
    tracing::error!(err = ?error, "display.show_floating_surface failed");
}

/// Logs that the display showed a floating surface.
fn log_show_succeeded() {
    tracing::debug!("display.show_floating_surface succeeded");
}

/// Logs that the floating surface `kind` is about to be hidden.
fn log_hide_requested(kind: &FloatingKind) {
    tracing::debug!(
        ?kind,
        "Received DisplayCommand::HideFloatingSurface, calling display.hide_floating_surface"
    );
}

/// Logs that the display refused to hide a floating surface.
fn log_hide_failed(error: &DisplayServerError) {
    tracing::error!(err = ?error, "display.hide_floating_surface failed");
}

/// Logs that the display hid a floating surface.
fn log_hide_succeeded() {
    tracing::debug!("display.hide_floating_surface succeeded");
}

/// Logs that `child_id` lost its rect on `monitor_id` because its site was withdrawn.
fn log_child_withdrawn(child_id: ModuleId, monitor_id: &MonitorId) {
    tracing::trace!(
        child = %child_id,
        monitor = %monitor_id,
        "Embedding site withdrawn; suspending child on this monitor"
    );
}

/// Logs that the module at `key` was lazily spawned under `parent` as `id`.
fn log_lazily_spawned(key: &crate::shared::primitives::ModuleKey, parent: ModuleId, id: ModuleId) {
    tracing::info!(module = %key, parent = %parent, id = %id, "Lazily spawned module");
}

/// Logs that the module at `key` could not be lazily spawned under `parent`.
fn log_lazy_spawn_failed(
    key: &crate::shared::primitives::ModuleKey,
    parent: ModuleId,
    error: &crate::features::module_runtime::ports::RegistryLoadError,
) {
    tracing::warn!(module = %key, parent = %parent, err = %error, "Lazy module spawn failed");
}

/// Logs that `action` is about to be triggered on the systray item `id`.
fn log_systray_requested(
    id: &crate::features::systray::domain::SystrayId,
    action: &crate::features::systray::domain::SystrayActionName,
    pos: Option<crate::shared::primitives::geometry::Position>,
) {
    tracing::debug!(
        ?id,
        ?action,
        ?pos,
        "Received UiCommand::SystrayAction, triggering SNI action"
    );
}

/// Logs that the systray host triggered `action` on the item `id`.
fn log_systray_succeeded(
    id: &crate::features::systray::domain::SystrayId,
    action: &crate::features::systray::domain::SystrayActionName,
) {
    tracing::debug!(?id, ?action, "SNI trigger_action succeeded");
}

/// Logs that the systray host could not trigger `action` on the item `id`.
fn log_systray_failed(
    id: &crate::features::systray::domain::SystrayId,
    action: &crate::features::systray::domain::SystrayActionName,
    error: &crate::features::systray::ports::SniPortError,
) {
    tracing::error!(?id, ?action, err = ?error, "SNI trigger_action failed");
}

/// Logs that the stylesheet `sheet_name` changed and its modules are about to reload.
fn log_reloading_style(sheet_name: &crate::features::styling::domain::StyleSheetName) {
    tracing::info!("Reloading style: {sheet_name}");
}

/// Logs that the base stylesheet changed, so every module is about to reload.
fn log_base_sheet_changed() {
    tracing::debug!("Base stylesheet changed; reloading all active modules");
}

/// Logs the modules about to reload because `sheet_name`, which they use, changed.
fn log_reloading_dependents(
    sheet_name: &crate::features::styling::domain::StyleSheetName,
    mods: &[crate::shared::primitives::ModuleName],
) {
    tracing::debug!(
        stylesheet = %sheet_name.as_str(),
        dependent_modules = ?mods.iter().map(crate::shared::primitives::ModuleName::as_str).collect::<Vec<_>>(),
        "Reloading modules dependent on modified stylesheet"
    );
}

/// Logs that the module called `name` is about to reload.
fn log_reloading_module(name: &crate::shared::primitives::ModuleName) {
    tracing::info!("Reloading module: {name}");
}

/// Logs that the module called `name` could not be reloaded.
fn log_reload_failed(
    name: &crate::shared::primitives::ModuleName,
    error: &crate::features::module_runtime::ports::RegistryLoadError,
) {
    tracing::error!("Failed to reload module {name}: {error}");
}

/// Logs that a new configuration arrived and the modules are about to reload from it.
fn log_config_reloading() {
    tracing::info!("Config hot-reload triggered in App");
}

/// Logs that the registry could not load the new configuration.
fn log_registry_reload_failed(error: &crate::features::module_runtime::ports::RegistryLoadError) {
    tracing::error!("Failed to reload registry on config change: {error}");
}

#[cfg(test)]
mod tests;
