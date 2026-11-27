//! The application actor: `CrankyApp` owns the module registry and the read model, and routes
//! display, layout, UI and system commands between the modules and the Wayland adapter.

use crate::app::commands::SystemCommand;
use crate::features::layout_engine::domain::DisplayCommand;
use crate::features::module_runtime::ports::LayoutEvent;
use crate::features::vdom::domain::UiCommand;
use crate::shared::config::domain::Config;
use crate::shared::events::signals::SignalHub;
use crate::shared::primitives::{ChildReport, ModuleId, MonitorId};
use crate::shared::wayland::ports::DisplayServerPort;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;

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
        deps: crate::features::module_runtime::ports::ModuleRuntimeDependencies<F, LS, DS, US>,
        channels: StateChannels,
        mut registry: Box<R>,
    ) -> Result<Self, AppError> {
        let config = deps.handles.hub().config_rx().borrow().clone();
        registry.load(&config).map_err(AppError::Module)?;

        let root_module = registry.root_module();
        let module_ids = registry.module_ids().to_vec();
        let module_names = registry.module_names();
        let name_to_ids = registry.name_to_ids().clone();
        let module_keys = registry.module_keys();
        let read_model = AppReadModel {
            config,
            root_module,
            module_ids,
            module_names,
            name_to_ids,
            module_keys,
            module_sizes: HashMap::new(),
            computed_layouts: HashMap::new(),
        };

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

    fn handle_display_commands(
        &mut self,
        initial: DisplayCommand,
        display: &mut impl DisplayServerPort,
    ) {
        let mut needs_render = false;
        let mut cmd = initial;
        loop {
            match cmd {
                DisplayCommand::RequestRender => {
                    needs_render = true;
                }
                DisplayCommand::ShowFloatingSurface(floating) => {
                    tracing::debug!(
                        kind = ?floating.kind,
                        monitor_id = ?floating.monitor_id,
                        anchor_rect = ?floating.anchor_rect,
                        offset = ?floating.offset,
                        parent = ?floating.parent,
                        "Received DisplayCommand::ShowFloatingSurface, calling display.show_floating_surface"
                    );
                    if let Err(error) = display.show_floating_surface(floating) {
                        tracing::error!(err = ?error, "display.show_floating_surface failed");
                    } else {
                        tracing::debug!("display.show_floating_surface succeeded");
                    }
                }
                DisplayCommand::HideFloatingSurface { kind } => {
                    tracing::debug!(
                        ?kind,
                        "Received DisplayCommand::HideFloatingSurface, calling display.hide_floating_surface"
                    );
                    if let Err(error) = display.hide_floating_surface(&kind) {
                        tracing::error!(err = ?error, "display.hide_floating_surface failed");
                    } else {
                        tracing::debug!("display.hide_floating_surface succeeded");
                    }
                }
            }
            if let Ok(next) = self.display_rx.try_recv() {
                cmd = next;
            } else {
                break;
            }
        }
        if needs_render {
            let _ = display.render_all(&self.read_model, &self.layout_senders);
        }
    }

    async fn handle_container_layouts(
        &mut self,
        parent: ModuleOnMonitor<'_>,
        layouts: &[crate::shared::primitives::ChildModuleLayout],
    ) {
        let ModuleOnMonitor {
            module: parent_id,
            monitor: monitor_id,
        } = parent;
        let mut seen = std::collections::HashSet::new();
        for child_layout in layouts {
            match self
                .registry
                .resolve_site(Some(parent_id), child_layout.key())
            {
                Some(child_id) => {
                    seen.insert(child_id);
                    let bounds = child_layout.bounds();
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
                None => {
                    self.ensure_lazy_spawn(
                        crate::features::module_runtime::ports::LazySite::new(
                            parent_id,
                            child_layout.key().clone(),
                        )
                        .with_options(child_layout.options().clone())
                        .with_surface(child_layout.surface()),
                    )
                    .await;
                }
            }
        }

        self.suspend_stale_children(parent, &seen);
    }

    /// Sends `child_id` its full, current cross-monitor bounds map, read back
    /// from `computed_layouts` rather than passed in — the same map both
    /// `handle_container_layouts` (a child gained or moved on a monitor) and
    /// `suspend_stale_children` (a child lost a monitor) need to resend.
    fn resend_child_bounds(&self, child_id: ModuleId) {
        let Some(sender) = self.layout_senders.get(&child_id) else {
            return;
        };
        let mut child_monitors = HashMap::new();
        for (mon, mod_map) in &self.read_model.computed_layouts {
            if let Some(&bounds) = mod_map.get(&child_id) {
                child_monitors.insert(mon.clone(), bounds);
            }
        }
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
        let ModuleOnMonitor {
            module: parent_id,
            monitor: monitor_id,
        } = parent;
        let Some(mod_map) = self.read_model.computed_layouts.get(monitor_id) else {
            return;
        };
        let stale: Vec<ModuleId> = mod_map
            .keys()
            .copied()
            .filter(|id| !seen.contains(id) && self.registry.parent_of(*id) == Some(parent_id))
            .collect();

        for child_id in stale {
            if let Some(mod_map) = self.read_model.computed_layouts.get_mut(monitor_id) {
                mod_map.remove(&child_id);
            }
            tracing::trace!(
                child = %child_id,
                monitor = %monitor_id,
                "Embedding site withdrawn; suspending child on this monitor"
            );
            self.resend_child_bounds(child_id);
        }
    }

    /// Rendering a `Module` node with no resolved site is the lazy-discovery
    /// signal (decision 2): spawn it here, on the spot, rather than waiting
    /// for a config stanza that will never come. Idempotent two ways — a
    /// site that already resolves is never re-spawned, and a site with a
    /// previously recorded terminal failure is never retried until a hot
    /// reload clears it — so a popup left open behind a ticking parent
    /// doesn't retry a typo'd module name every frame.
    async fn ensure_lazy_spawn(&mut self, site: crate::features::module_runtime::ports::LazySite) {
        let parent = site.parent();
        let module_site =
            crate::shared::primitives::ModuleSite::new(Some(parent), site.key().clone());
        let key = module_site.key();
        if self
            .hub()
            .module_errors_rx()
            .borrow()
            .contains_key(&module_site)
        {
            return;
        }

        if self.would_cycle(parent, key) {
            self.record_module_error(
                module_site,
                "would create a cycle with an ancestor".to_string(),
            );
            return;
        }

        match self.registry.spawn_module(
            site,
            crate::features::module_runtime::ports::ModuleSetup::new(
                &self.read_model.config,
                &self.deps,
            ),
        ) {
            Ok(spawned) => {
                let id = spawned.id();
                tracing::info!(module = %key, parent = %parent, id = %id, "Lazily spawned module");
                self.layout_senders.insert(id, spawned.into_sender());
                self.read_model.module_ids = self.registry.module_ids().to_vec();
                self.read_model.module_names = self.registry.module_names();
                self.read_model
                    .name_to_ids
                    .clone_from(self.registry.name_to_ids());
                self.read_model.module_keys = self.registry.module_keys();

                if let Some(supervisor) = self.adapter_supervisor.as_mut() {
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
            }
            Err(error) => {
                tracing::warn!(module = %key, parent = %parent, err = %error, "Lazy module spawn failed");
                self.record_module_error(module_site, error.to_string());
            }
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
        let mut current = Some(parent);
        while let Some(id) = current {
            if module_keys.get(&id) == Some(key) {
                return true;
            }
            current = self.registry.parent_of(id);
        }
        false
    }

    fn record_module_error(&self, site: crate::shared::primitives::ModuleSite, reason: String) {
        let mut errors = self.hub().module_errors_rx().borrow().clone();
        errors.insert(site, reason);
        let _ = self.hub().module_errors_tx().send(errors);
    }

    async fn handle_layout_events(
        &mut self,
        initial: LayoutEvent,
        display: &mut impl DisplayServerPort,
    ) {
        let mut event = initial;
        loop {
            match event {
                LayoutEvent::ContainerLayoutsCalculated {
                    parent_id,
                    monitor_id,
                    layouts,
                } => {
                    let parent = ModuleOnMonitor {
                        module: parent_id,
                        monitor: &monitor_id,
                    };
                    self.handle_container_layouts(parent, &layouts).await;
                }
                LayoutEvent::ModuleSizeChanged {
                    monitor_id,
                    module_id,
                    size,
                    settled_width,
                } => {
                    let module = ModuleOnMonitor {
                        module: module_id,
                        monitor: &monitor_id,
                    };
                    self.handle_size_changed(module, ChildReport::new(size, settled_width));
                }
            }
            if let Ok(next) = self.layout_rx.try_recv() {
                event = next;
            } else {
                break;
            }
        }
        let _ = display.render_all(&self.read_model, &self.layout_senders);
    }

    async fn handle_ui_commands(
        &mut self,
        initial: UiCommand,
        sni: &impl crate::features::systray::ports::SniPort,
    ) {
        let mut cmd = initial;
        loop {
            match cmd {
                UiCommand::Exec(cmd_str) => {
                    tracing::debug!("Executing shell command: {cmd_str}");
                    let _ = std::process::Command::new("sh")
                        .arg("-c")
                        .arg(cmd_str)
                        .spawn();
                }
                UiCommand::SystrayAction { id, action, pos } => {
                    tracing::debug!(
                        ?id,
                        ?action,
                        ?pos,
                        "Received UiCommand::SystrayAction, triggering SNI action"
                    );
                    let call = crate::features::systray::domain::SystrayActionCall::new(
                        action.clone(),
                        pos,
                    );
                    match sni.trigger_action(&id, &call).await {
                        Ok(()) => tracing::debug!(?id, ?action, "SNI trigger_action succeeded"),
                        Err(error) => {
                            tracing::error!(?id, ?action, err = ?error, "SNI trigger_action failed");
                        }
                    }
                }
            }
            if let Ok(next) = self.ui_rx.try_recv() {
                cmd = next;
            } else {
                break;
            }
        }
    }

    fn handle_reload_style(
        &mut self,
        sheet_name: &crate::features::styling::domain::StyleSheetName,
        deps: &crate::features::module_runtime::ports::ModuleRuntimeDependencies<F, LS, DS, US>,
    ) -> bool {
        tracing::info!("Reloading style: {sheet_name}");
        if sheet_name.as_str() == "base" {
            tracing::debug!("Base stylesheet changed; reloading all active modules");
            let all_modules: Vec<_> = self.read_model.module_names.values().cloned().collect();
            for mod_name in all_modules {
                if let Ok(new_senders) = self.registry.reload_module(
                    &mod_name,
                    crate::features::module_runtime::ports::ModuleSetup::new(
                        &self.read_model.config,
                        deps,
                    ),
                ) {
                    for (id, sender) in new_senders {
                        self.layout_senders.insert(id, sender);
                    }
                }
            }
            true
        } else {
            let mods = self.registry.modules_using_style(sheet_name);
            tracing::debug!(
                stylesheet = %sheet_name.as_str(),
                dependent_modules = ?mods.iter().map(crate::shared::primitives::ModuleName::as_str).collect::<Vec<_>>(),
                "Reloading modules dependent on modified stylesheet"
            );
            let mut any_reloaded = false;
            for mod_name in mods {
                match self.registry.reload_module(
                    &mod_name,
                    crate::features::module_runtime::ports::ModuleSetup::new(
                        &self.read_model.config,
                        deps,
                    ),
                ) {
                    Ok(new_senders) => {
                        for (id, sender) in new_senders {
                            self.layout_senders.insert(id, sender);
                        }
                        any_reloaded = true;
                    }
                    Err(error) => tracing::error!("Failed to reload module {mod_name}: {error}"),
                }
            }
            any_reloaded
        }
    }

    fn handle_system_commands(
        &mut self,
        initial: SystemCommand,
        display: &mut impl DisplayServerPort,
    ) {
        let mut needs_render = false;
        let mut cmd = initial;
        let deps = self.deps.clone();
        loop {
            match cmd {
                SystemCommand::ReloadModule(name) => {
                    tracing::info!("Reloading module: {name}");
                    match self.registry.reload_module(
                        &name,
                        crate::features::module_runtime::ports::ModuleSetup::new(
                            &self.read_model.config,
                            &deps,
                        ),
                    ) {
                        Ok(new_senders) => {
                            for (id, sender) in new_senders {
                                self.layout_senders.insert(id, sender);
                            }
                            needs_render = true;
                        }
                        Err(error) => tracing::error!("Failed to reload module {name}: {error}"),
                    }
                }
                SystemCommand::ReloadStyle(sheet_name) => {
                    if self.handle_reload_style(&sheet_name, &deps) {
                        needs_render = true;
                    }
                }
            }
            if let Ok(next) = self.system_rx.try_recv() {
                cmd = next;
            } else {
                break;
            }
        }
        if needs_render {
            let _ = display.render_all(&self.read_model, &self.layout_senders);
        }
    }

    fn handle_config_changed(&mut self, new_config: Config) {
        tracing::info!("Config hot-reload triggered in App");
        self.read_model.config = new_config;
        self.read_model.module_sizes.clear();

        self.registry.clear();
        if let Err(error) = self.registry.load(&self.read_model.config) {
            tracing::error!("Failed to reload registry on config change: {error}");
        } else {
            self.read_model.root_module = self.registry.root_module();
            self.read_model.module_ids = self.registry.module_ids().to_vec();
            self.read_model.module_names = self.registry.module_names();
            self.read_model
                .name_to_ids
                .clone_from(self.registry.name_to_ids());
            self.read_model.module_keys = self.registry.module_keys();
            self.layout_senders = self.registry.spawn_all(&self.deps);
        }
    }

    fn handle_hyprland_changed(
        &mut self,
        state: &crate::shared::events::signals::HyprlandState,
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
        mut display: impl DisplayServerPort,
        bus: BusAdapters<impl crate::features::systray::ports::SniPort>,
    ) -> Result<(), AppError> {
        let BusAdapters {
            mut subscriptions,
            systray: sni,
        } = bus;
        let mut config_rx = self.hub().config_rx();
        let mut hyprland_rx = self.hub().hyprland_rx();
        let mut monitor_scales_rx = self.hub().monitor_scales_rx();

        self.registry
            .register_dbus_subscriptions(&mut subscriptions)
            .await;

        loop {
            let _ = display.flush();

            tokio::select! {
                res = display.wait_for_events() => {
                    res.map_err(|error| AppError::Internal { message: error.to_string() })?;
                    display.dispatch_pending().map_err(|error| AppError::Internal { message: error.to_string() })?;
                }
                Some(display_cmd) = self.display_rx.recv() => {
                    self.handle_display_commands(display_cmd, &mut display);
                }
                Some(layout_event) = self.layout_rx.recv() => {
                    self.handle_layout_events(layout_event, &mut display).await;
                }
                Some(ui_cmd) = self.ui_rx.recv() => {
                    self.handle_ui_commands(ui_cmd, &sni).await;
                }
                Some(sys_cmd) = self.system_rx.recv() => {
                    self.handle_system_commands(sys_cmd, &mut display);
                }
                Ok(()) = config_rx.changed() => {
                    let new_config = config_rx.borrow().clone();
                    self.handle_config_changed(new_config);
                }
                Ok(()) = hyprland_rx.changed() => {
                    let state = hyprland_rx.borrow().clone();
                    self.handle_hyprland_changed(&state, &mut display);
                }
                Ok(()) = monitor_scales_rx.changed() => {
                    self.prune_removed_monitors();
                }
            }
        }
    }

    fn handle_size_changed(&mut self, module: ModuleOnMonitor<'_>, report: ChildReport) {
        let ModuleOnMonitor {
            module: module_id,
            monitor: monitor_id,
        } = module;
        let size = *report.size();
        let key = self.read_model.module_keys.get(&module_id).cloned();
        tracing::trace!(monitor = %monitor_id, module = %module_id, ?size, ?key, "handle_size_changed called");
        self.read_model
            .module_sizes
            .entry(monitor_id.clone())
            .or_default()
            .insert(module_id, size);

        if let Some(key) = key {
            let mut sizes_map = self.hub().module_sizes_rx().borrow().clone();
            let mon_entry = sizes_map.entry(monitor_id.clone()).or_default();
            mon_entry.insert_report(key, report);
            let _ = self.hub().module_sizes_tx().send(sizes_map);
        }
    }
}

#[cfg(test)]
mod tests;
