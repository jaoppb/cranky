use crate::app::commands::SystemCommand;
use crate::features::layout_engine::domain::DisplayCommand;
use crate::features::module_runtime::ports::LayoutEvent;
use crate::features::vdom::domain::UiCommand;
use crate::shared::config::domain::Config;
use crate::shared::events::signals::SignalHub;
use crate::shared::primitives::{ModuleId, MonitorId, geometry::Size};
use crate::shared::wayland::ports::DisplayServerPort;
use crate::shared::wayland::ports::DynSurfaceManager;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Module error: {0}")]
    Module(#[from] crate::features::module_runtime::ports::RegistryLoadError),
    #[error("Internal error: {message}")]
    Internal { message: String },
}

pub use crate::shared::wayland::domain::{AppReadModel, ModuleLayout};

pub struct CrankyApp<
    R: crate::features::module_runtime::ports::ModuleRegistryPort<F, LS, DS, US> + 'static,
    F: crate::shared::rendering::ports::canvas::CanvasFactory + 'static,
    LS: crate::features::module_runtime::ports::LayoutEventSender + 'static,
    DS: crate::features::layout_engine::domain::DisplayCommandSender + 'static,
    US: crate::features::vdom::domain::UiCommandSender + 'static,
> {
    hub: Arc<SignalHub>,
    read_model: AppReadModel,
    display_rx: mpsc::Receiver<DisplayCommand>,
    layout_rx: mpsc::Receiver<LayoutEvent>,
    ui_rx: mpsc::Receiver<UiCommand>,
    system_rx: mpsc::Receiver<SystemCommand>,
    layout_senders:
        HashMap<ModuleId, Box<dyn crate::features::module_runtime::ports::LayoutSender>>,
    surface_manager: DynSurfaceManager,
    display_sender: Arc<DS>,
    layout_sender: Arc<LS>,
    ui_sender: Arc<US>,
    registry: Box<R>,
    canvas_factory: F,
    /// `None` until `set_adapter_supervisor` is called — main.rs wires it in
    /// once the (fallible, boot-fatal) `DBus` connection is established, which
    /// happens after `CrankyApp::new`. A lazy spawn before that point simply
    /// can't ensure any adapter; nothing in v1 lazily spawns before boot
    /// finishes, so this is never actually observed empty in practice.
    adapter_supervisor: Option<crate::app::adapter_supervisor::AdapterSupervisor>,
}

pub struct StateServices<F, LS, DS, US> {
    hub: Arc<SignalHub>,
    surface_manager: DynSurfaceManager,
    canvas_factory: F,
    display_sender: Arc<DS>,
    layout_sender: Arc<LS>,
    ui_sender: Arc<US>,
}

impl<F, LS, DS, US> StateServices<F, LS, DS, US> {
    #[must_use]
    pub const fn new(
        hub: Arc<SignalHub>,
        surface_manager: DynSurfaceManager,
        canvas_factory: F,
        display_sender: Arc<DS>,
        layout_sender: Arc<LS>,
        ui_sender: Arc<US>,
    ) -> Self {
        Self {
            hub,
            surface_manager,
            canvas_factory,
            display_sender,
            layout_sender,
            ui_sender,
        }
    }
}

#[allow(clippy::struct_field_names)]
pub struct StateChannels {
    display_rx: mpsc::Receiver<DisplayCommand>,
    layout_rx: mpsc::Receiver<LayoutEvent>,
    ui_rx: mpsc::Receiver<UiCommand>,
    system_rx: mpsc::Receiver<SystemCommand>,
}

impl StateChannels {
    #[must_use]
    pub const fn new(
        display_rx: mpsc::Receiver<DisplayCommand>,
        layout_rx: mpsc::Receiver<LayoutEvent>,
        ui_rx: mpsc::Receiver<UiCommand>,
        system_rx: mpsc::Receiver<SystemCommand>,
    ) -> Self {
        Self {
            display_rx,
            layout_rx,
            ui_rx,
            system_rx,
        }
    }
}

impl<
    R: crate::features::module_runtime::ports::ModuleRegistryPort<F, LS, DS, US> + 'static,
    F: crate::shared::rendering::ports::canvas::CanvasFactory + 'static,
    LS: crate::features::module_runtime::ports::LayoutEventSender + 'static,
    DS: crate::features::layout_engine::domain::DisplayCommandSender + 'static,
    US: crate::features::vdom::domain::UiCommandSender + 'static,
> CrankyApp<R, F, LS, DS, US>
{
    /// Creates a new [`CrankyApp`] instance and initializes modules from config.
    ///
    /// # Errors
    ///
    /// Returns [`AppError::Module`] if initial module loading fails.
    pub fn new(
        config: Config,
        services: StateServices<F, LS, DS, US>,
        channels: StateChannels,
        mut registry: Box<R>,
    ) -> Result<Self, AppError> {
        registry.load(&config).map_err(AppError::Module)?;

        let root_module = registry.root_module();
        let module_ids = registry.module_ids().to_vec();
        let module_names = registry.module_names().clone();
        let name_to_ids = registry.name_to_ids().clone();
        let module_keys = registry.module_keys().clone();
        let deps = crate::features::module_runtime::ports::ModuleRuntimeDependencies::new(
            services.hub.clone(),
            services.surface_manager.clone(),
            services.layout_sender.clone(),
            services.display_sender.clone(),
            services.ui_sender.clone(),
            services.canvas_factory.clone(),
        );
        let layout_senders = registry.spawn_all(&deps);

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

        Ok(Self {
            hub: services.hub,
            read_model,
            display_rx: channels.display_rx,
            layout_rx: channels.layout_rx,
            ui_rx: channels.ui_rx,
            system_rx: channels.system_rx,
            layout_senders,
            surface_manager: services.surface_manager,
            display_sender: services.display_sender,
            layout_sender: services.layout_sender,
            ui_sender: services.ui_sender,
            registry,
            canvas_factory: services.canvas_factory,
            adapter_supervisor: None,
        })
    }

    /// Wires in the on-demand adapter starter (decision 3). Called from
    /// `main.rs` once the `DBus` connection it needs is established — after
    /// `CrankyApp::new`, since that connection failing is fatal at boot and
    /// must not be entangled with module loading.
    pub fn set_adapter_supervisor(
        &mut self,
        supervisor: crate::app::adapter_supervisor::AdapterSupervisor,
    ) {
        self.adapter_supervisor = Some(supervisor);
    }

    #[must_use]
    pub fn active_signals(
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
                DisplayCommand::ShowFloatingSurface {
                    kind,
                    monitor_id,
                    anchor_rect,
                    buffer,
                    logical_size,
                    offset,
                } => {
                    tracing::debug!(
                        ?kind,
                        ?monitor_id,
                        ?anchor_rect,
                        ?offset,
                        "Received DisplayCommand::ShowFloatingSurface, calling display.show_floating_surface"
                    );
                    if let Err(e) = display.show_floating_surface(
                        kind,
                        monitor_id,
                        anchor_rect,
                        buffer,
                        logical_size,
                        offset,
                    ) {
                        tracing::error!(err = ?e, "display.show_floating_surface failed");
                    } else {
                        tracing::debug!("display.show_floating_surface succeeded");
                    }
                }
                DisplayCommand::HideFloatingSurface { kind } => {
                    tracing::debug!(
                        ?kind,
                        "Received DisplayCommand::HideFloatingSurface, calling display.hide_floating_surface"
                    );
                    if let Err(e) = display.hide_floating_surface(&kind) {
                        tracing::error!(err = ?e, "display.hide_floating_surface failed");
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
        parent_id: ModuleId,
        monitor_id: &MonitorId,
        layouts: &[crate::shared::primitives::ChildModuleLayout],
    ) {
        for child_layout in layouts {
            match self
                .registry
                .resolve_site(Some(parent_id), child_layout.key())
            {
                Some(child_id) => {
                    let bounds = crate::shared::primitives::ChildBounds::new(
                        *child_layout.bounds(),
                        child_layout.constraint(),
                    );
                    self.read_model
                        .computed_layouts
                        .entry(monitor_id.clone())
                        .or_default()
                        .insert(child_id, bounds);

                    tracing::trace!(
                        child = %child_id,
                        monitor = %monitor_id,
                        bounds = ?child_layout.bounds(),
                        "Updating computed_layouts for child module"
                    );
                    if let Some(sender) = self.layout_senders.get(&child_id) {
                        let mut child_monitors = HashMap::new();
                        for (mon, mod_map) in &self.read_model.computed_layouts {
                            if let Some(&bounds) = mod_map.get(&child_id) {
                                child_monitors.insert(mon.clone(), bounds);
                            }
                        }
                        sender.send_layout(child_monitors);
                    }
                }
                None => {
                    self.ensure_lazy_spawn(
                        parent_id,
                        child_layout.key(),
                        child_layout.options().clone(),
                    )
                    .await;
                }
            }
        }
    }

    /// Rendering a `Module` node with no resolved site is the lazy-discovery
    /// signal (decision 2): spawn it here, on the spot, rather than waiting
    /// for a config stanza that will never come. Idempotent two ways — a
    /// site that already resolves is never re-spawned, and a site with a
    /// previously recorded terminal failure is never retried until a hot
    /// reload clears it — so a popup left open behind a ticking parent
    /// doesn't retry a typo'd module name every frame.
    async fn ensure_lazy_spawn(
        &mut self,
        parent: ModuleId,
        key: &crate::shared::primitives::ModuleKey,
        options: crate::shared::primitives::ModuleOptions,
    ) {
        let site = crate::shared::primitives::ModuleSite::new(Some(parent), key.clone());
        if self.hub.module_errors_rx().borrow().contains_key(&site) {
            return;
        }

        if self.would_cycle(parent, key.name()) {
            self.record_module_error(site, "would create a cycle with an ancestor".to_string());
            return;
        }

        let deps = crate::features::module_runtime::ports::ModuleRuntimeDependencies::new(
            self.hub.clone(),
            self.surface_manager.clone(),
            self.layout_sender.clone(),
            self.display_sender.clone(),
            self.ui_sender.clone(),
            self.canvas_factory.clone(),
        );

        match self.registry.spawn_module(
            parent,
            key.name(),
            key.instance_id().cloned(),
            options,
            &self.read_model.config,
            &deps,
        ) {
            Ok(spawned) => {
                let id = spawned.id();
                tracing::info!(module = %key, parent = %parent, id = %id, "Lazily spawned module");
                self.layout_senders.insert(id, spawned.into_sender());
                self.read_model.module_ids = self.registry.module_ids().to_vec();
                self.read_model
                    .module_names
                    .clone_from(self.registry.module_names());
                self.read_model
                    .name_to_ids
                    .clone_from(self.registry.name_to_ids());
                self.read_model
                    .module_keys
                    .clone_from(self.registry.module_keys());

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
            Err(e) => {
                tracing::warn!(module = %key, parent = %parent, err = %e, "Lazy module spawn failed");
                self.record_module_error(site, e.to_string());
            }
        }
    }

    fn would_cycle(&self, parent: ModuleId, name: &crate::shared::primitives::ModuleName) -> bool {
        let mut current = Some(parent);
        while let Some(id) = current {
            if self.registry.module_names().get(&id) == Some(name) {
                return true;
            }
            current = self.registry.parent_of(id);
        }
        false
    }

    fn record_module_error(&self, site: crate::shared::primitives::ModuleSite, reason: String) {
        let mut errors = self.hub.module_errors_rx().borrow().clone();
        errors.insert(site, reason);
        let _ = self.hub.module_errors_tx().send(errors);
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
                    self.handle_container_layouts(parent_id, &monitor_id, &layouts)
                        .await;
                }
                LayoutEvent::ChildModuleSizeChanged {
                    parent_id: _,
                    child_key,
                    monitor_id,
                    size,
                } => {
                    let mut sizes_map = self.hub.module_sizes_rx().borrow().clone();
                    let mon_entry = sizes_map.entry(monitor_id).or_default();
                    mon_entry.insert(child_key, size);
                    let _ = self.hub.module_sizes_tx().send(sizes_map);
                }
                LayoutEvent::ModuleSizeChanged {
                    monitor_id,
                    module_id,
                    size,
                } => {
                    self.handle_size_changed(&monitor_id, module_id, size);
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
                    let _ = std::process::Command::new("sh").arg("-c").arg(cmd_str).spawn();
                }
                UiCommand::SystrayAction { id, action, pos } => {
                    tracing::debug!(?id, ?action, ?pos, "Received UiCommand::SystrayAction, triggering SNI action");
                    match sni.trigger_action(&id, &action, pos).await {
                        Ok(()) => tracing::debug!(?id, ?action, "SNI trigger_action succeeded"),
                        Err(e) => tracing::error!(?id, ?action, err = ?e, "SNI trigger_action failed"),
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
                if let Ok(new_senders) = self.registry.reload_module(&mod_name, &self.read_model.config, deps) {
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
                match self.registry.reload_module(&mod_name, &self.read_model.config, deps) {
                    Ok(new_senders) => {
                        for (id, sender) in new_senders {
                            self.layout_senders.insert(id, sender);
                        }
                        any_reloaded = true;
                    }
                    Err(e) => tracing::error!("Failed to reload module {mod_name}: {e}"),
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
        let deps = crate::features::module_runtime::ports::ModuleRuntimeDependencies::new(
            self.hub.clone(),
            self.surface_manager.clone(),
            self.layout_sender.clone(),
            self.display_sender.clone(),
            self.ui_sender.clone(),
            self.canvas_factory.clone(),
        );
        loop {
            match cmd {
                SystemCommand::ReloadModule(name) => {
                    tracing::info!("Reloading module: {name}");
                    match self.registry.reload_module(&name, &self.read_model.config, &deps) {
                        Ok(new_senders) => {
                            for (id, sender) in new_senders {
                                self.layout_senders.insert(id, sender);
                            }
                            needs_render = true;
                        }
                        Err(e) => tracing::error!("Failed to reload module {name}: {e}"),
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
        if let Err(e) = self.registry.load(&self.read_model.config) {
            tracing::error!("Failed to reload registry on config change: {e}");
        } else {
            self.read_model.root_module = self.registry.root_module();
            self.read_model.module_ids = self.registry.module_ids().to_vec();
            self.read_model.module_names.clone_from(self.registry.module_names());
            self.read_model.name_to_ids.clone_from(self.registry.name_to_ids());
            self.read_model.module_keys.clone_from(self.registry.module_keys());
            let deps = crate::features::module_runtime::ports::ModuleRuntimeDependencies::new(
                self.hub.clone(),
                self.surface_manager.clone(),
                self.layout_sender.clone(),
                self.display_sender.clone(),
                self.ui_sender.clone(),
                self.canvas_factory.clone(),
            );
            self.layout_senders = self.registry.spawn_all(&deps);
        }
    }

    fn handle_hyprland_changed(
        &self,
        state: &crate::shared::events::signals::HyprlandState,
        current_focused_monitor: &mut String,
        display: &mut impl DisplayServerPort,
    ) {
        let new_focused = state
            .focused_monitor()
            .map(|n| n.as_str().to_string())
            .unwrap_or_default();

        if new_focused != *current_focused_monitor {
            *current_focused_monitor = new_focused;
            let _ = display.render_all(&self.read_model, &self.layout_senders);
        }
    }

    /// Runs the main event loop, listening for display events, commands, and signals.
    ///
    /// # Errors
    ///
    /// Returns [`AppError::Internal`] if display server communication or event dispatching fails.
    pub async fn run(
        &mut self,
        mut display: impl DisplayServerPort,
        mut dbus: crate::shared::dbus::subscription_manager::DbusSubscriptionManager,
        sni: impl crate::features::systray::ports::SniPort,
    ) -> Result<(), AppError> {
        let mut config_rx = self.hub.config_rx();
        let mut hyprland_rx = self.hub.hyprland_rx();

        self.registry.register_dbus_subscriptions(&mut dbus).await;

        let mut current_focused_monitor = String::new();

        loop {
            let _ = display.flush();

            tokio::select! {
                res = display.wait_for_events() => {
                    res.map_err(|e| AppError::Internal { message: e.to_string() })?;
                    display.dispatch_pending().map_err(|e| AppError::Internal { message: e.to_string() })?;
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
                    self.handle_hyprland_changed(&state, &mut current_focused_monitor, &mut display);
                }
            }
        }
    }

    pub fn handle_size_changed(&mut self, monitor_id: &MonitorId, module_id: ModuleId, size: Size) {
        let key = self.read_model.module_keys.get(&module_id).cloned();
        tracing::trace!(monitor = %monitor_id, module = %module_id, ?size, ?key, "handle_size_changed called");
        self.read_model
            .module_sizes
            .entry(monitor_id.clone())
            .or_default()
            .insert(module_id, size);

        if let Some(key) = key {
            let mut sizes_map = self.hub.module_sizes_rx().borrow().clone();
            let mon_entry = sizes_map.entry(monitor_id.clone()).or_default();
            mon_entry.insert(key, size);
            let _ = self.hub.module_sizes_tx().send(sizes_map);
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::similar_names)]
    use super::*;
    use crate::features::module_runtime::ports::MockModuleRegistryPort;
    use crate::shared::primitives::geometry::{BarWidth, Position, Rect};
    use crate::shared::wayland::ports::MockDisplayServerPort;
    use crate::shared::wayland::ports::MockSurfaceManagerPort;
    use std::sync::Arc;
    use tokio::sync::mpsc;

    type TestMockRegistry = MockModuleRegistryPort<
        crate::shared::rendering::adapters::tiny_skia::TinySkiaCanvasFactory,
        tokio::sync::mpsc::Sender<LayoutEvent>,
        tokio::sync::mpsc::Sender<DisplayCommand>,
        tokio::sync::mpsc::Sender<UiCommand>,
    >;

    #[tokio::test]
    async fn test_app_initialization() {
        let config = Config::default();
        let hub = Arc::new(SignalHub::new(config.clone()));
        let (display_tx, display_rx) = mpsc::channel(32);
        let (layout_tx, layout_rx) = mpsc::channel(32);
        let (ui_tx, ui_rx) = mpsc::channel(32);
        let (_system_tx, system_rx) = mpsc::channel(32);

        let surface_manager: DynSurfaceManager = Arc::new(MockSurfaceManagerPort::new());

        let mut mock_registry = TestMockRegistry::new();
        mock_registry.expect_load().returning(|_| Ok(()));
        mock_registry.expect_root_module().return_const(None);
        mock_registry.expect_module_ids().return_const(Vec::new());
        mock_registry
            .expect_module_names()
            .return_const(HashMap::new());
        mock_registry
            .expect_name_to_ids()
            .return_const(HashMap::new());
        mock_registry
            .expect_module_keys()
            .return_const(HashMap::new());
        mock_registry
            .expect_spawn_all()
            .returning(|_| HashMap::new());

        let canvas_factory =
            crate::shared::rendering::adapters::tiny_skia::TinySkiaCanvasFactory::new();

        let display_sender = Arc::new(display_tx);
        let layout_sender = Arc::new(layout_tx);
        let ui_sender = Arc::new(ui_tx);

        let services = StateServices::new(
            hub,
            surface_manager,
            canvas_factory,
            display_sender,
            layout_sender,
            ui_sender,
        );
        let channels = StateChannels::new(
            display_rx,
            layout_rx,
            ui_rx,
            system_rx,
        );
        let app_result = CrankyApp::new(
            config,
            services,
            channels,
            Box::new(mock_registry),
        );

        assert!(app_result.is_ok());
    }

    #[tokio::test]
    async fn test_app_run_exit_on_display_error() {
        let config = Config::default();
        let hub = Arc::new(SignalHub::new(config.clone()));
        let (display_tx, display_rx) = mpsc::channel(32);
        let (layout_tx, layout_rx) = mpsc::channel(32);
        let (ui_tx, ui_rx) = mpsc::channel(32);
        let (_system_tx, system_rx) = mpsc::channel(32);

        let surface_manager: DynSurfaceManager = Arc::new(MockSurfaceManagerPort::new());

        let mut mock_registry = TestMockRegistry::new();
        mock_registry.expect_load().returning(|_| Ok(()));
        mock_registry.expect_root_module().return_const(None);
        mock_registry.expect_module_ids().return_const(Vec::new());
        mock_registry
            .expect_module_names()
            .return_const(HashMap::new());
        mock_registry
            .expect_name_to_ids()
            .return_const(HashMap::new());
        mock_registry
            .expect_module_keys()
            .return_const(HashMap::new());
        mock_registry
            .expect_spawn_all()
            .returning(|_| HashMap::new());
        mock_registry
            .expect_register_dbus_subscriptions()
            .returning(|_| Box::pin(std::future::ready(())));
        mock_registry.expect_clear().returning(|| ());

        let canvas_factory =
            crate::shared::rendering::adapters::tiny_skia::TinySkiaCanvasFactory::new();

        let display_sender = Arc::new(display_tx);
        let layout_sender = Arc::new(layout_tx);
        let ui_sender = Arc::new(ui_tx);

        let services = StateServices::new(
            hub.clone(),
            surface_manager,
            canvas_factory,
            display_sender,
            layout_sender,
            ui_sender,
        );
        let channels = StateChannels::new(
            display_rx,
            layout_rx,
            ui_rx,
            system_rx,
        );
        let mut app = CrankyApp::new(
            config,
            services,
            channels,
            Box::new(mock_registry),
        )
        .unwrap();

        let mut mock_display = MockDisplayServerPort::new();
        mock_display.expect_flush().returning(|| Ok(()));
        mock_display.expect_wait_for_events().returning(|| {
            Box::pin(std::future::ready(Err(
                crate::shared::wayland::ports::DisplayServerError::Internal("Test error".into()),
            )))
        });

        let mock_conn = crate::shared::dbus::ports::MockDbusConnectionPort::new();
        let mock_dbus = crate::shared::dbus::subscription_manager::DbusSubscriptionManager::new(
            std::sync::Arc::new(mock_conn),
            &hub,
        );
        let mock_sni = crate::features::systray::ports::MockSniPort::new();

        let result = app.run(mock_display, mock_dbus, mock_sni).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_calculate_layout_unfocused() {
        let unfocused = crate::shared::config::domain::PartialRootConfig::new(
            crate::shared::config::domain::CreatePartialRootConfigCommand::new(
                Some(crate::shared::primitives::geometry::BarHeight::new(20)),
                None,
                None,
            ),
        );
        let mut opts_map = HashMap::new();
        opts_map.insert(
            "center".to_string(),
            crate::shared::primitives::DynamicValue::Array(vec![
                crate::shared::primitives::DynamicValue::String("clock".to_string()),
            ]),
        );
        let opts = crate::shared::primitives::ModuleOptions::new(opts_map);

        let root_config = crate::shared::config::domain::RootConfig::new(
            crate::shared::config::domain::CreateRootConfigCommand::new(
                crate::shared::primitives::ModuleName::new("bar"),
                crate::shared::primitives::geometry::BarHeight::new(30),
                crate::shared::config::domain::VerticalAlignment::Center,
                crate::shared::config::domain::MarginConfig::default(),
                Some(unfocused),
                opts,
            ),
        );

        let config = Config::new(
            root_config,
            crate::shared::config::domain::ModulesConfig::default(),
            crate::shared::config::domain::RenderingMode::default(),
            crate::features::metrics::domain::MetricsConfig::default(),
            crate::shared::config::domain::TooltipConfig::default(),
            crate::shared::config::domain::PopupConfig::default(),
        );

        let mut name_to_ids = HashMap::new();
        name_to_ids.insert(
            crate::shared::primitives::ModuleName::new("clock"),
            vec![crate::shared::primitives::ModuleId::new(1)],
        );

        let read_model = AppReadModel {
            config: config.clone(),
            root_module: None,
            module_ids: vec![crate::shared::primitives::ModuleId::new(1)],
            module_names: {
                let mut m = HashMap::new();
                m.insert(
                    crate::shared::primitives::ModuleId::new(1),
                    crate::shared::primitives::ModuleName::new("clock"),
                );
                m
            },
            name_to_ids,
            module_keys: HashMap::new(),
            module_sizes: {
                let mut m = HashMap::new();
                let mut s = HashMap::new();
                s.insert(
                    crate::shared::primitives::ModuleId::new(1),
                    crate::shared::primitives::geometry::Size::new(50, 10),
                );
                m.insert(MonitorId::new("DP-1"), s);
                m
            },
            computed_layouts: HashMap::new(),
        };

        let monitor_1 = MonitorId::new("DP-1");

        // 1. Calculate with focused config
        let layouts_focused =
            read_model.calculate_layout(&monitor_1, BarWidth::new(1920), config.root());
        assert_eq!(layouts_focused.len(), 1);
        let layout_focused = &layouts_focused[0];

        // height 30, available height = 30, module height = 10, y should be (30 - 10) / 2 = 10
        assert_eq!(layout_focused.bounds().position().y(), 10);

        // 2. Calculate with unfocused config
        let unfocused_root = config.root().as_unfocused();
        let layouts_unfocused =
            read_model.calculate_layout(&monitor_1, BarWidth::new(1920), &unfocused_root);
        assert_eq!(layouts_unfocused.len(), 1);
        let layout_unfocused = &layouts_unfocused[0];

        // height 20, available height = 20, module height = 10, y should be (20 - 10) / 2 = 5
        assert_eq!(layout_unfocused.bounds().position().y(), 5);
    }

    #[test]
    fn test_app_error_fmt() {
        let err1 = AppError::Module(
            crate::features::module_runtime::ports::RegistryLoadError::ModuleNotFound(
                "test".into(),
            ),
        );
        assert_eq!(err1.to_string(), "Module error: Module not found: test");
        let err2 = AppError::Internal {
            message: "test".into(),
        };
        assert_eq!(err2.to_string(), "Internal error: test");
    }

    #[test]
    fn test_calculate_layout_left_right() {
        let mut opts_map = HashMap::new();
        opts_map.insert(
            "left".to_string(),
            crate::shared::primitives::DynamicValue::Array(vec![
                crate::shared::primitives::DynamicValue::String("m1".to_string()),
                crate::shared::primitives::DynamicValue::String("m2".to_string()),
            ]),
        );
        opts_map.insert(
            "right".to_string(),
            crate::shared::primitives::DynamicValue::Array(vec![
                crate::shared::primitives::DynamicValue::String("m3".to_string()),
            ]),
        );
        let opts = crate::shared::primitives::ModuleOptions::new(opts_map);

        let root_config = crate::shared::config::domain::RootConfig::new(
            crate::shared::config::domain::CreateRootConfigCommand::new(
                crate::shared::primitives::ModuleName::new("bar"),
                crate::shared::primitives::geometry::BarHeight::new(30),
                crate::shared::config::domain::VerticalAlignment::Center,
                crate::shared::config::domain::MarginConfig::default(),
                None,
                opts,
            ),
        );

        let config = Config::new(
            root_config,
            crate::shared::config::domain::ModulesConfig::default(),
            crate::shared::config::domain::RenderingMode::default(),
            crate::features::metrics::domain::MetricsConfig::default(),
            crate::shared::config::domain::TooltipConfig::default(),
            crate::shared::config::domain::PopupConfig::default(),
        );

        let mut name_to_ids = HashMap::new();
        name_to_ids.insert(
            crate::shared::primitives::ModuleName::new("m1"),
            vec![ModuleId::new(1)],
        );
        name_to_ids.insert(
            crate::shared::primitives::ModuleName::new("m2"),
            vec![ModuleId::new(2)],
        );
        name_to_ids.insert(
            crate::shared::primitives::ModuleName::new("m3"),
            vec![ModuleId::new(3)],
        );

        let mut read_model = AppReadModel {
            config: config.clone(),
            root_module: None,
            module_ids: vec![ModuleId::new(1), ModuleId::new(2), ModuleId::new(3)],
            module_names: HashMap::new(),
            name_to_ids,
            module_keys: HashMap::new(),
            module_sizes: HashMap::new(),
            computed_layouts: HashMap::new(),
        };

        let mut sizes = HashMap::new();
        sizes.insert(ModuleId::new(1), Size::new(100, 20));
        sizes.insert(ModuleId::new(2), Size::new(50, 20));
        sizes.insert(ModuleId::new(3), Size::new(80, 20));
        read_model
            .module_sizes
            .insert(MonitorId::new("DP-1"), sizes);

        let layouts = read_model.calculate_layout(
            &MonitorId::new("DP-1"),
            BarWidth::new(1920),
            config.root(),
        );
        assert_eq!(layouts.len(), 3);

        let gap = 8;
        let padding_h = 8;

        let l1 = layouts.iter().find(|l| l.id() == ModuleId::new(1)).unwrap();
        assert_eq!(l1.bounds().x(), padding_h);

        let l2 = layouts.iter().find(|l| l.id() == ModuleId::new(2)).unwrap();
        assert_eq!(l2.bounds().x(), padding_h + 100 + gap);

        let l3 = layouts.iter().find(|l| l.id() == ModuleId::new(3)).unwrap();
        assert_eq!(l3.bounds().x(), 1920 - padding_h - 80);
    }

    #[allow(clippy::too_many_lines)]
    #[tokio::test]
    async fn test_app_run_commands_and_signals() {
        let config = Config::default();
        let hub = Arc::new(SignalHub::new(config.clone()));
        let (display_tx, display_rx) = mpsc::channel(32);
        let (layout_tx, layout_rx) = mpsc::channel(32);
        let (ui_tx, ui_rx) = mpsc::channel(32);
        let (_system_tx, system_rx) = mpsc::channel(32);

        let surface_manager: DynSurfaceManager = Arc::new(MockSurfaceManagerPort::new());

        let mut mock_registry = TestMockRegistry::new();
        mock_registry.expect_load().returning(|_| Ok(()));
        mock_registry.expect_root_module().return_const(None);
        mock_registry.expect_module_ids().return_const(Vec::new());
        mock_registry
            .expect_module_names()
            .return_const(HashMap::new());
        mock_registry
            .expect_name_to_ids()
            .return_const(HashMap::new());
        mock_registry
            .expect_module_keys()
            .return_const(HashMap::new());
        mock_registry
            .expect_spawn_all()
            .returning(|_| HashMap::new());
        mock_registry
            .expect_register_dbus_subscriptions()
            .returning(|_| Box::pin(std::future::ready(())));
        mock_registry.expect_clear().returning(|| ());

        let canvas_factory =
            crate::shared::rendering::adapters::tiny_skia::TinySkiaCanvasFactory::new();

        let display_sender = Arc::new(display_tx.clone());
        let layout_sender = Arc::new(layout_tx.clone());
        let ui_sender = Arc::new(ui_tx.clone());

        let services = StateServices::new(
            hub.clone(),
            surface_manager,
            canvas_factory,
            display_sender,
            layout_sender,
            ui_sender,
        );
        let channels = StateChannels::new(
            display_rx,
            layout_rx,
            ui_rx,
            system_rx,
        );
        let mut app = CrankyApp::new(
            config,
            services,
            channels,
            Box::new(mock_registry),
        )
        .unwrap();

        let mut mock_display = MockDisplayServerPort::new();
        mock_display.expect_flush().returning(|| Ok(()));

        // Let it succeed twice, then fail to exit the loop
        let mut call_count = 0;
        mock_display.expect_wait_for_events().returning(move || {
            call_count += 1;
            if call_count <= 2 {
                Box::pin(std::future::ready(Ok(())))
            } else {
                Box::pin(std::future::ready(Err(
                    crate::shared::wayland::ports::DisplayServerError::Internal("Exit".into()),
                )))
            }
        });
        mock_display.expect_dispatch_pending().returning(|| Ok(()));
        mock_display.expect_render_all().returning(|_, _| Ok(()));
        mock_display
            .expect_show_floating_surface()
            .returning(|_, _, _, _, _, _| Ok(()));
        mock_display
            .expect_hide_floating_surface()
            .returning(|_| Ok(()));

        let mock_conn = crate::shared::dbus::ports::MockDbusConnectionPort::new();
        let mock_dbus = crate::shared::dbus::subscription_manager::DbusSubscriptionManager::new(
            std::sync::Arc::new(mock_conn),
            &hub,
        );
        let mut mock_sni = crate::features::systray::ports::MockSniPort::new();
        mock_sni.expect_trigger_action().returning(|_, _, _| Ok(()));

        // Queue commands
        display_tx
            .send(DisplayCommand::RequestRender)
            .await
            .unwrap();
        layout_tx
            .send(LayoutEvent::ModuleSizeChanged {
                monitor_id: MonitorId::new("1"),
                module_id: ModuleId::new(1),
                size: Size::new(10, 10),
            })
            .await
            .unwrap();
        display_tx
            .send(DisplayCommand::ShowFloatingSurface {
                kind: crate::features::layout_engine::domain::FloatingKind::Tooltip,
                monitor_id: None,
                anchor_rect: None,
                buffer: crate::shared::primitives::render::RenderBuffer::new(
                    vec![0u8; 4],
                    Size::new(1, 1),
                ),
                logical_size: Size::new(1, 1),
                offset: None,
            })
            .await
            .unwrap();
        display_tx
            .send(DisplayCommand::HideFloatingSurface {
                kind: crate::features::layout_engine::domain::FloatingKind::Tooltip,
            })
            .await
            .unwrap();
        ui_tx
            .send(UiCommand::SystrayAction {
                id: "a".into(),
                action: "b".into(),
                pos: None,
            })
            .await
            .unwrap();

        // Trigger config and hyprland changes
        hub.config_tx().send(Config::default()).unwrap();
        hub.hyprland_tx()
            .send(crate::shared::events::signals::HyprlandState::new(
                std::collections::BTreeMap::new(),
                std::collections::BTreeMap::new(),
                Some(crate::features::workspaces::domain::MonitorName::new("1")),
            ))
            .unwrap();

        let result = app.run(mock_display, mock_dbus, mock_sni).await;
        assert!(result.is_err());
    }

    #[allow(clippy::too_many_lines)]
    #[tokio::test]
    async fn test_container_layouts_calculated_preserves_multi_monitors() {
        let config = Config::default();
        let hub = Arc::new(SignalHub::new(config.clone()));
        let (display_tx, display_rx) = mpsc::channel(32);
        let (layout_tx, layout_rx) = mpsc::channel(32);
        let (ui_tx, ui_rx) = mpsc::channel(32);
        let (_system_tx, system_rx) = mpsc::channel(32);
        let surface_manager: DynSurfaceManager = Arc::new(MockSurfaceManagerPort::new());

        let mut mock_registry = TestMockRegistry::new();
        mock_registry.expect_load().returning(|_| Ok(()));
        mock_registry
            .expect_root_module()
            .return_const(Some(ModuleId::new(0)));
        mock_registry
            .expect_module_ids()
            .return_const(vec![ModuleId::new(0), ModuleId::new(1)]);
        let mut names = HashMap::new();
        names.insert(
            ModuleId::new(0),
            crate::shared::primitives::ModuleName::new("bar"),
        );
        names.insert(
            ModuleId::new(1),
            crate::shared::primitives::ModuleName::new("clock"),
        );
        mock_registry.expect_module_names().return_const(names);
        let mut name_to_ids = HashMap::new();
        name_to_ids.insert(
            crate::shared::primitives::ModuleName::new("bar"),
            vec![ModuleId::new(0)],
        );
        name_to_ids.insert(
            crate::shared::primitives::ModuleName::new("clock"),
            vec![ModuleId::new(1)],
        );
        mock_registry.expect_name_to_ids().return_const(name_to_ids);
        mock_registry
            .expect_module_keys()
            .return_const(HashMap::new());
        mock_registry.expect_resolve_site().returning(|parent, key| {
            if parent == Some(ModuleId::new(0)) && key.name().as_str() == "clock" {
                Some(ModuleId::new(1))
            } else {
                None
            }
        });

        let (layout_tx_0, _layout_rx_0) = tokio::sync::watch::channel(HashMap::new());
        let (layout_tx_1, mut layout_rx_1) = tokio::sync::watch::channel(HashMap::new());
        let mut senders: HashMap<
            ModuleId,
            Box<dyn crate::features::module_runtime::ports::LayoutSender>,
        > = HashMap::new();
        senders.insert(
            ModuleId::new(0),
            Box::new(crate::app::registry::WatchLayoutSender::new(layout_tx_0)),
        );
        senders.insert(
            ModuleId::new(1),
            Box::new(crate::app::registry::WatchLayoutSender::new(layout_tx_1)),
        );

        mock_registry.expect_spawn_all().return_once(|_| senders);
        mock_registry
            .expect_register_dbus_subscriptions()
            .returning(|_| Box::pin(std::future::ready(())));
        mock_registry.expect_clear().returning(|| ());

        let canvas_factory =
            crate::shared::rendering::adapters::tiny_skia::TinySkiaCanvasFactory::new();

        let display_sender = Arc::new(display_tx);
        let layout_sender = Arc::new(layout_tx.clone());
        let ui_sender = Arc::new(ui_tx);

        let services = StateServices::new(
            hub.clone(),
            surface_manager,
            canvas_factory,
            display_sender,
            layout_sender,
            ui_sender,
        );
        let channels = StateChannels::new(
            display_rx,
            layout_rx,
            ui_rx,
            system_rx,
        );
        let mut app = CrankyApp::new(
            config,
            services,
            channels,
            Box::new(mock_registry),
        )
        .unwrap();

        // 1. Send ContainerLayoutsCalculated for DP-1
        layout_tx
            .send(LayoutEvent::ContainerLayoutsCalculated {
                parent_id: ModuleId::new(0),
                monitor_id: MonitorId::new("DP-1"),
                layouts: vec![crate::shared::primitives::ChildModuleLayout::new(
                    crate::shared::primitives::ModuleKey::from_name(
                        crate::shared::primitives::ModuleName::new("clock"),
                    ),
                    Rect::new(Position::new(100, 0), Size::new(80, 24)),
                    crate::shared::primitives::SizeConstraint::none(),
                    crate::shared::primitives::ModuleOptions::default(),
                )],
            })
            .await
            .unwrap();

        // 2. Send ContainerLayoutsCalculated for DP-2
        layout_tx
            .send(LayoutEvent::ContainerLayoutsCalculated {
                parent_id: ModuleId::new(0),
                monitor_id: MonitorId::new("DP-2"),
                layouts: vec![crate::shared::primitives::ChildModuleLayout::new(
                    crate::shared::primitives::ModuleKey::from_name(
                        crate::shared::primitives::ModuleName::new("clock"),
                    ),
                    Rect::new(Position::new(150, 0), Size::new(80, 24)),
                    crate::shared::primitives::SizeConstraint::none(),
                    crate::shared::primitives::ModuleOptions::default(),
                )],
            })
            .await
            .unwrap();

        let mut mock_display = MockDisplayServerPort::new();
        mock_display.expect_flush().returning(|| Ok(()));
        let (stop_tx, stop_rx) = tokio::sync::watch::channel(false);
        mock_display.expect_wait_for_events().returning(move || {
            let mut stop_rx = stop_rx.clone();
            Box::pin(async move {
                let _ = stop_rx.changed().await;
                Err(crate::shared::wayland::ports::DisplayServerError::Internal(
                    "Done".to_string(),
                ))
            })
        });
        mock_display.expect_dispatch_pending().returning(|| Ok(()));
        mock_display.expect_render_all().returning(|_, _| Ok(()));
        mock_display
            .expect_show_floating_surface()
            .returning(|_, _, _, _, _, _| Ok(()));
        mock_display
            .expect_hide_floating_surface()
            .returning(|_| Ok(()));

        let mock_conn = crate::shared::dbus::ports::MockDbusConnectionPort::new();
        let mock_dbus = crate::shared::dbus::subscription_manager::DbusSubscriptionManager::new(
            std::sync::Arc::new(mock_conn),
            &hub,
        );
        let mut mock_sni = crate::features::systray::ports::MockSniPort::new();
        mock_sni.expect_trigger_action().returning(|_, _, _| Ok(()));

        // Run app in background task and verify layout_rx_1 gets both DP-1 and DP-2
        let app_handle = tokio::spawn(async move {
            let _ = app.run(mock_display, mock_dbus, mock_sni).await;
        });

        // Wait for layout_rx_1 to see both DP-1 and DP-2
        let mut attempts = 0;
        loop {
            layout_rx_1.changed().await.unwrap();
            let current = layout_rx_1.borrow().clone();
            if current.contains_key(&MonitorId::new("DP-1"))
                && current.contains_key(&MonitorId::new("DP-2"))
            {
                assert_eq!(current.get(&MonitorId::new("DP-1")).unwrap().rect().x(), 100);
                assert_eq!(current.get(&MonitorId::new("DP-2")).unwrap().rect().x(), 150);
                break;
            }
            attempts += 1;
            assert!(
                attempts <= 10,
                "Did not receive both DP-1 and DP-2 bounds in child layout_rx"
            );
        }

        let _ = stop_tx.send(true);
        let _ = app_handle.await;
    }

    #[tokio::test]
    async fn test_handle_container_layouts_does_not_cross_talk_between_sites() {
        // Two different parents (10 and 20) each embed a module named
        // "calendar" — resolve_site must route each parent's layout event to
        // its own site's ModuleId, never the other parent's.
        let config = Config::default();
        let hub = Arc::new(SignalHub::new(config.clone()));
        let (display_tx, display_rx) = mpsc::channel(32);
        let (layout_tx, layout_rx) = mpsc::channel(32);
        let (ui_tx, ui_rx) = mpsc::channel(32);
        let (_system_tx, system_rx) = mpsc::channel(32);
        let surface_manager: DynSurfaceManager = Arc::new(MockSurfaceManagerPort::new());

        let parent_10 = ModuleId::new(10);
        let parent_20 = ModuleId::new(20);
        let site_a = ModuleId::new(11);
        let site_b = ModuleId::new(21);

        let mut mock_registry = TestMockRegistry::new();
        mock_registry.expect_load().returning(|_| Ok(()));
        mock_registry.expect_root_module().return_const(None);
        mock_registry.expect_module_ids().return_const(Vec::new());
        mock_registry
            .expect_module_names()
            .return_const(HashMap::new());
        mock_registry
            .expect_name_to_ids()
            .return_const(HashMap::new());
        mock_registry
            .expect_module_keys()
            .return_const(HashMap::new());
        mock_registry.expect_resolve_site().returning(move |parent, key| {
            if key.name().as_str() != "calendar" {
                return None;
            }
            if parent == Some(parent_10) {
                Some(site_a)
            } else if parent == Some(parent_20) {
                Some(site_b)
            } else {
                None
            }
        });
        mock_registry
            .expect_spawn_all()
            .returning(|_| HashMap::new());

        let canvas_factory =
            crate::shared::rendering::adapters::tiny_skia::TinySkiaCanvasFactory::new();

        let services = StateServices::new(
            hub,
            surface_manager,
            canvas_factory,
            Arc::new(display_tx),
            Arc::new(layout_tx),
            Arc::new(ui_tx),
        );
        let channels = StateChannels::new(display_rx, layout_rx, ui_rx, system_rx);
        let mut app = CrankyApp::new(config, services, channels, Box::new(mock_registry)).unwrap();

        let monitor = MonitorId::new("DP-1");
        let key = crate::shared::primitives::ModuleKey::from_name(
            crate::shared::primitives::ModuleName::new("calendar"),
        );

        app.handle_container_layouts(
            parent_10,
            &monitor,
            &[crate::shared::primitives::ChildModuleLayout::new(
                key.clone(),
                Rect::new(Position::new(10, 0), Size::new(50, 20)),
                crate::shared::primitives::SizeConstraint::none(),
                crate::shared::primitives::ModuleOptions::default(),
            )],
        )
        .await;
        app.handle_container_layouts(
            parent_20,
            &monitor,
            &[crate::shared::primitives::ChildModuleLayout::new(
                key,
                Rect::new(Position::new(90, 0), Size::new(50, 20)),
                crate::shared::primitives::SizeConstraint::none(),
                crate::shared::primitives::ModuleOptions::default(),
            )],
        )
        .await;

        let layouts = app.read_model.computed_layouts.get(&monitor).unwrap();
        assert_eq!(layouts.get(&site_a).unwrap().rect().x(), 10);
        assert_eq!(layouts.get(&site_b).unwrap().rect().x(), 90);
        assert_eq!(layouts.len(), 2);
    }

    #[tokio::test]
    async fn test_ensure_lazy_spawn_rejects_self_cycle() {
        // A parent named "calendar" embedding another "calendar" would
        // create a cycle — must be rejected and recorded without ever
        // calling `spawn_module` (not stubbed, so a call would panic).
        let config = Config::default();
        let hub = Arc::new(SignalHub::new(config.clone()));
        let hub_check = hub.clone();
        let (display_tx, display_rx) = mpsc::channel(32);
        let (layout_tx, layout_rx) = mpsc::channel(32);
        let (ui_tx, ui_rx) = mpsc::channel(32);
        let (_system_tx, system_rx) = mpsc::channel(32);
        let surface_manager: DynSurfaceManager = Arc::new(MockSurfaceManagerPort::new());

        let parent = ModuleId::new(1);
        let mut names = HashMap::new();
        names.insert(
            parent,
            crate::shared::primitives::ModuleName::new("calendar"),
        );

        let mut mock_registry = TestMockRegistry::new();
        mock_registry.expect_load().returning(|_| Ok(()));
        mock_registry.expect_root_module().return_const(None);
        mock_registry.expect_module_ids().return_const(Vec::new());
        mock_registry.expect_module_names().return_const(names);
        mock_registry
            .expect_name_to_ids()
            .return_const(HashMap::new());
        mock_registry
            .expect_module_keys()
            .return_const(HashMap::new());
        mock_registry.expect_resolve_site().returning(|_, _| None);
        mock_registry
            .expect_spawn_all()
            .returning(|_| HashMap::new());

        let canvas_factory =
            crate::shared::rendering::adapters::tiny_skia::TinySkiaCanvasFactory::new();
        let services = StateServices::new(
            hub,
            surface_manager,
            canvas_factory,
            Arc::new(display_tx),
            Arc::new(layout_tx),
            Arc::new(ui_tx),
        );
        let channels = StateChannels::new(display_rx, layout_rx, ui_rx, system_rx);
        let mut app = CrankyApp::new(config, services, channels, Box::new(mock_registry)).unwrap();

        let monitor = MonitorId::new("DP-1");
        let key = crate::shared::primitives::ModuleKey::from_name(
            crate::shared::primitives::ModuleName::new("calendar"),
        );
        app.handle_container_layouts(
            parent,
            &monitor,
            &[crate::shared::primitives::ChildModuleLayout::new(
                key,
                Rect::new(Position::new(0, 0), Size::new(50, 20)),
                crate::shared::primitives::SizeConstraint::none(),
                crate::shared::primitives::ModuleOptions::default(),
            )],
        )
        .await;

        let site = crate::shared::primitives::ModuleSite::new(
            Some(parent),
            crate::shared::primitives::ModuleKey::from_name(
                crate::shared::primitives::ModuleName::new("calendar"),
            ),
        );
        assert!(hub_check.module_errors_rx().borrow().contains_key(&site));
        assert!(app.read_model.computed_layouts.is_empty());
    }

    #[tokio::test]
    async fn test_ensure_lazy_spawn_does_not_retry_after_recorded_failure() {
        let config = Config::default();
        let hub = Arc::new(SignalHub::new(config.clone()));
        let hub_check = hub.clone();
        let (display_tx, display_rx) = mpsc::channel(32);
        let (layout_tx, layout_rx) = mpsc::channel(32);
        let (ui_tx, ui_rx) = mpsc::channel(32);
        let (_system_tx, system_rx) = mpsc::channel(32);
        let surface_manager: DynSurfaceManager = Arc::new(MockSurfaceManagerPort::new());

        let parent = ModuleId::new(1);
        let mut names = HashMap::new();
        names.insert(parent, crate::shared::primitives::ModuleName::new("bar"));

        let mut mock_registry = TestMockRegistry::new();
        mock_registry.expect_load().returning(|_| Ok(()));
        mock_registry.expect_root_module().return_const(None);
        mock_registry.expect_module_ids().return_const(Vec::new());
        mock_registry.expect_module_names().return_const(names);
        mock_registry
            .expect_name_to_ids()
            .return_const(HashMap::new());
        mock_registry
            .expect_module_keys()
            .return_const(HashMap::new());
        mock_registry.expect_resolve_site().returning(|_, _| None);
        mock_registry.expect_parent_of().returning(|_| None);
        mock_registry
            .expect_spawn_all()
            .returning(|_| HashMap::new());
        mock_registry.expect_spawn_module().times(1).returning(|_, _, _, _, _, _| {
            Err(
                crate::features::module_runtime::ports::RegistryLoadError::ModuleNotFound(
                    crate::shared::primitives::ModuleName::new("calendarr"),
                ),
            )
        });

        let canvas_factory =
            crate::shared::rendering::adapters::tiny_skia::TinySkiaCanvasFactory::new();
        let services = StateServices::new(
            hub,
            surface_manager,
            canvas_factory,
            Arc::new(display_tx),
            Arc::new(layout_tx),
            Arc::new(ui_tx),
        );
        let channels = StateChannels::new(display_rx, layout_rx, ui_rx, system_rx);
        let mut app = CrankyApp::new(config, services, channels, Box::new(mock_registry)).unwrap();

        let monitor = MonitorId::new("DP-1");
        let key = crate::shared::primitives::ModuleKey::from_name(
            crate::shared::primitives::ModuleName::new("calendarr"),
        );
        let layouts = [crate::shared::primitives::ChildModuleLayout::new(
            key,
            Rect::new(Position::new(0, 0), Size::new(50, 20)),
            crate::shared::primitives::SizeConstraint::none(),
            crate::shared::primitives::ModuleOptions::default(),
        )];

        // First attempt: spawn_module is called and fails, and the failure
        // is recorded.
        app.handle_container_layouts(parent, &monitor, &layouts)
            .await;
        let site = crate::shared::primitives::ModuleSite::new(
            Some(parent),
            crate::shared::primitives::ModuleKey::from_name(
                crate::shared::primitives::ModuleName::new("calendarr"),
            ),
        );
        assert!(hub_check.module_errors_rx().borrow().contains_key(&site));

        // Second identical attempt: spawn_module must not be called again —
        // `.times(1)` on the mock enforces this; a second call would panic.
        app.handle_container_layouts(parent, &monitor, &layouts)
            .await;
    }
}
