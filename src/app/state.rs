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
        })
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
                    layout,
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
                        *layout,
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

    fn handle_container_layouts(
        &mut self,
        monitor_id: &MonitorId,
        layouts: &[crate::shared::primitives::ChildModuleLayout],
    ) {
        for child_layout in layouts {
            if let Some(ids) = self.read_model.name_to_ids.get(child_layout.key().name())
                && let Some(&child_id) = ids.first()
            {
                self.read_model
                    .computed_layouts
                    .entry(monitor_id.clone())
                    .or_default()
                    .insert(child_id, *child_layout.bounds());

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
        }
    }

    fn handle_layout_events(&mut self, initial: LayoutEvent, display: &mut impl DisplayServerPort) {
        let mut event = initial;
        loop {
            match event {
                LayoutEvent::ContainerLayoutsCalculated {
                    parent_id: _,
                    monitor_id,
                    layouts,
                } => {
                    self.handle_container_layouts(&monitor_id, &layouts);
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
                    match sni.trigger_action(&id, &action, pos).await {
                        Ok(()) => tracing::debug!(?id, ?action, "SNI trigger_action succeeded"),
                        Err(e) => {
                            tracing::error!(?id, ?action, err = ?e, "SNI trigger_action failed");
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
                if let Ok(new_senders) =
                    self.registry
                        .reload_module(&mod_name, &self.read_model.config, deps)
                {
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
                match self
                    .registry
                    .reload_module(&mod_name, &self.read_model.config, deps)
                {
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
                    match self
                        .registry
                        .reload_module(&name, &self.read_model.config, &deps)
                    {
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
            self.read_model
                .module_names
                .clone_from(self.registry.module_names());
            self.read_model
                .name_to_ids
                .clone_from(self.registry.name_to_ids());
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
            .hub
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

        let mut sizes_map = self.hub.module_sizes_rx().borrow().clone();
        let before = sizes_map.len();
        sizes_map.retain(|id, _| live.contains(id));
        if sizes_map.len() != before {
            let _ = self.hub.module_sizes_tx().send(sizes_map);
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
        let mut monitor_scales_rx = self.hub.monitor_scales_rx();

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
                    self.handle_layout_events(layout_event, &mut display);
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
                Ok(()) = monitor_scales_rx.changed() => {
                    self.prune_removed_monitors();
                }
            }
        }
    }

    pub fn handle_size_changed(&mut self, monitor_id: &MonitorId, module_id: ModuleId, size: Size) {
        let name = self.read_model.module_names.get(&module_id).cloned();
        tracing::trace!(monitor = %monitor_id, module = %module_id, ?size, ?name, "handle_size_changed called");
        self.read_model
            .module_sizes
            .entry(monitor_id.clone())
            .or_default()
            .insert(module_id, size);

        if let Some(name) = name {
            let mut sizes_map = self.hub.module_sizes_rx().borrow().clone();
            let mon_entry = sizes_map.entry(monitor_id.clone()).or_default();
            mon_entry.insert(crate::shared::primitives::ModuleKey::new(name, None), size);
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
        let channels = StateChannels::new(display_rx, layout_rx, ui_rx, system_rx);
        let app_result = CrankyApp::new(config, services, channels, Box::new(mock_registry));

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
        let channels = StateChannels::new(display_rx, layout_rx, ui_rx, system_rx);
        let mut app = CrankyApp::new(config, services, channels, Box::new(mock_registry)).unwrap();

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
        let channels = StateChannels::new(display_rx, layout_rx, ui_rx, system_rx);
        let mut app = CrankyApp::new(config, services, channels, Box::new(mock_registry)).unwrap();

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
            .returning(|_, _, _, _, _| Ok(()));
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
                layout: Box::new(crate::features::layout_engine::domain::StyledNode::Text {
                    path: crate::features::layout_engine::domain::NodePath::root(),
                    node_key: None,
                    text: crate::features::vdom::domain::TextContent::new("t".to_string()),
                    style: crate::features::styling::domain::ComputedStyle::default(),
                    on_click: None,
                    on_hover: None,
                    tooltip: None,
                    popup: None,
                    panel: None,
                }),
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

        // Trigger config, hyprland, and monitor_scales changes
        hub.config_tx().send(Config::default()).unwrap();
        hub.hyprland_tx()
            .send(crate::shared::events::signals::HyprlandState::new(
                std::collections::BTreeMap::new(),
                std::collections::BTreeMap::new(),
                Some(crate::features::workspaces::domain::MonitorName::new("1")),
            ))
            .unwrap();
        let mut scales = hub.monitor_scales_rx().borrow().clone();
        scales.insert(
            MonitorId::new("1"),
            crate::shared::primitives::geometry::Scale::new(1.0),
        );
        hub.monitor_scales_tx().send(scales).unwrap();

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
        let channels = StateChannels::new(display_rx, layout_rx, ui_rx, system_rx);
        let mut app = CrankyApp::new(config, services, channels, Box::new(mock_registry)).unwrap();

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
            .returning(|_, _, _, _, _| Ok(()));
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
                assert_eq!(current.get(&MonitorId::new("DP-1")).unwrap().x(), 100);
                assert_eq!(current.get(&MonitorId::new("DP-2")).unwrap().x(), 150);
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

    /// A monitor Hyprland no longer reports must lose its per-monitor state
    /// everywhere it's kept — `read_model.module_sizes` /
    /// `computed_layouts`, and the shared `module_sizes` signal — while an
    /// unrelated live monitor's state is untouched. Without this, the
    /// module stays "discovered" forever (`discover_monitors` unions this
    /// state with Hyprland's live list) and keeps rendering for a monitor
    /// with no surface left.
    #[tokio::test]
    async fn test_prune_removed_monitors_drops_only_the_dead_monitor() {
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
            .expect_spawn_all()
            .returning(|_| HashMap::new());
        mock_registry
            .expect_register_dbus_subscriptions()
            .returning(|_| Box::pin(std::future::ready(())));
        mock_registry.expect_clear().returning(|| ());

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

        let live_mon = MonitorId::new("eDP-1");
        let dead_mon = MonitorId::new("HDMI-A-1");

        app.read_model
            .module_sizes
            .insert(dead_mon.clone(), HashMap::new());
        app.read_model
            .module_sizes
            .insert(live_mon.clone(), HashMap::new());
        app.read_model
            .computed_layouts
            .insert(dead_mon.clone(), HashMap::new());
        app.read_model
            .computed_layouts
            .insert(live_mon.clone(), HashMap::new());

        let mut shared_sizes = app.hub.module_sizes_rx().borrow().clone();
        shared_sizes.insert(
            dead_mon.clone(),
            crate::shared::primitives::layout::ChildSizesMap::new(),
        );
        shared_sizes.insert(
            live_mon.clone(),
            crate::shared::primitives::layout::ChildSizesMap::new(),
        );
        app.hub.module_sizes_tx().send(shared_sizes).unwrap();

        // Only "eDP-1" is still live per Wayland.
        let mut scales = app.hub.monitor_scales_rx().borrow().clone();
        scales.insert(
            live_mon.clone(),
            crate::shared::primitives::geometry::Scale::new(1.0),
        );
        app.hub.monitor_scales_tx().send(scales).unwrap();

        app.prune_removed_monitors();

        assert!(!app.read_model.module_sizes.contains_key(&dead_mon));
        assert!(app.read_model.module_sizes.contains_key(&live_mon));
        assert!(!app.read_model.computed_layouts.contains_key(&dead_mon));
        assert!(app.read_model.computed_layouts.contains_key(&live_mon));

        let after = app.hub.module_sizes_rx().borrow().clone();
        assert!(!after.contains_key(&dead_mon));
        assert!(after.contains_key(&live_mon));
    }
}
