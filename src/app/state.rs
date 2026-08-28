use crate::app::commands::AppCommand;
use crate::shared::config::domain::Config;
use crate::shared::events::signals::SignalHub;
use crate::shared::primitives::{
    ModuleId, MonitorId,
    geometry::{BarWidth, Position, Rect, Size},
};
use crate::shared::wayland::ports::DisplayServerPort;
use crate::shared::wayland::ports::DynSurfaceManager;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{error, info};

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Module error: {0}")]
    Module(#[from] crate::features::module_runtime::ports::RegistryLoadError),
    #[error("Internal error: {message}")]
    Internal { message: String },
}

pub struct ModuleLayout {
    id: ModuleId,
    bounds: Rect,
}

impl ModuleLayout {
    #[must_use]
    pub const fn id(&self) -> crate::shared::primitives::ModuleId {
        self.id
    }

    #[must_use]
    pub const fn bounds(&self) -> &Rect {
        &self.bounds
    }
}

pub struct AppReadModel {
    config: Config,
    root_module: Option<ModuleId>,
    module_ids: Vec<ModuleId>,
    module_names: HashMap<ModuleId, crate::shared::primitives::ModuleName>,
    name_to_ids: HashMap<crate::shared::primitives::ModuleName, Vec<ModuleId>>,
    module_sizes: HashMap<MonitorId, HashMap<ModuleId, Size>>,
    computed_layouts: HashMap<MonitorId, HashMap<ModuleId, Rect>>,
}

impl AppReadModel {
    #[must_use]
    pub const fn config(&self) -> &crate::shared::config::domain::Config {
        &self.config
    }

    #[must_use]
    pub const fn root_module(&self) -> Option<ModuleId> {
        self.root_module
    }

    #[allow(
        clippy::as_conversions,
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation
    )]
    #[must_use]
    pub fn calculate_layout(
        &self,
        monitor: &MonitorId,
        bar_width: BarWidth,
        root_config: &crate::shared::config::domain::RootConfig,
    ) -> Vec<ModuleLayout> {
        let mut layouts = Vec::new();
        let bar_height = root_config.height();
        let available_height = bar_height.value() as f32;

        if let Some(root_id) = self.root_module {
            layouts.push(ModuleLayout {
                id: root_id,
                bounds: Rect::new(
                    Position::new(0, 0),
                    Size::new(bar_width.value(), bar_height.value()),
                ),
            });
        }

        if let Some(mon_layouts) = self.computed_layouts.get(monitor) {
            for (&mod_id, &bounds) in mon_layouts {
                if Some(mod_id) != self.root_module {
                    layouts.push(ModuleLayout { id: mod_id, bounds });
                }
            }
            return layouts;
        }

        let get_size = |id: &ModuleId| {
            self.module_sizes
                .get(monitor)
                .and_then(|m| m.get(id))
                .copied()
                .unwrap_or(Size::new(0, 0))
        };

        let gap = 8.0f32;
        let padding_h = 8.0f32;

        let get_module_ids = |key: &str| -> Vec<ModuleId> {
            let mut ids = Vec::new();
            if let Some(crate::shared::primitives::DynamicValue::Array(arr)) =
                root_config.options().get(key)
            {
                for v in arr {
                    if let Some(name_str) = v.as_str() {
                        let mod_name = crate::shared::primitives::ModuleName::new(name_str);
                        if let Some(mod_ids) = self.name_to_ids.get(&mod_name) {
                            ids.extend(mod_ids.iter().copied());
                        }
                    }
                }
            }
            ids
        };

        let left_ids = get_module_ids("left");
        let center_ids = get_module_ids("center");
        let right_ids = get_module_ids("right");

        // Calculate left modules
        let mut left_x = padding_h;
        for id in left_ids {
            let size = get_size(&id);
            let y = (available_height - size.height() as f32).max(0.0) / 2.0;
            layouts.push(ModuleLayout {
                id,
                bounds: Rect::new(Position::new(left_x as i32, y as i32), size),
            });
            left_x += size.width() as f32 + gap;
        }

        // Calculate right modules
        let mut right_x = bar_width.value() as f32 - padding_h;
        let mut right_layouts = Vec::new();
        for id in right_ids.into_iter().rev() {
            let size = get_size(&id);
            right_x -= size.width() as f32;
            let y = (available_height - size.height() as f32).max(0.0) / 2.0;
            right_layouts.push(ModuleLayout {
                id,
                bounds: Rect::new(Position::new(right_x as i32, y as i32), size),
            });
            right_x -= gap;
        }
        layouts.extend(right_layouts.into_iter().rev());

        // Calculate center modules
        let mut center_width = 0.0;
        let mut center_sizes = Vec::new();
        for id in center_ids {
            let size = get_size(&id);
            center_width += size.width() as f32;
            center_sizes.push((id, size));
        }
        if !center_sizes.is_empty() {
            center_width =
                ((center_sizes.len().saturating_sub(1)) as f32).mul_add(gap, center_width);
        }

        let mut center_x = (bar_width.value() as f32 - center_width) / 2.0;
        for (id, size) in center_sizes {
            let y = (available_height - size.height() as f32).max(0.0) / 2.0;
            layouts.push(ModuleLayout {
                id,
                bounds: Rect::new(Position::new(center_x as i32, y as i32), size),
            });
            center_x += size.width() as f32 + gap;
        }

        layouts
    }
}

pub struct CrankyApp<
    R: crate::features::module_runtime::ports::ModuleRegistryPort<F> + 'static,
    F: crate::shared::rendering::ports::canvas::CanvasFactory + 'static,
> {
    hub: Arc<SignalHub>,
    read_model: AppReadModel,
    command_rx: mpsc::Receiver<AppCommand>,
    layout_senders:
        HashMap<ModuleId, Box<dyn crate::features::module_runtime::ports::LayoutSender>>,
    surface_manager: DynSurfaceManager,
    command_tx_clone: mpsc::Sender<AppCommand>,
    registry: Box<R>,
    canvas_factory: Arc<std::sync::Mutex<F>>,
}

struct MpscCommandSender(mpsc::Sender<AppCommand>);
impl crate::features::module_runtime::ports::CommandSender for MpscCommandSender {
    fn send_command(&self, cmd: AppCommand) {
        if let Err(e) = self.0.try_send(cmd) {
            tracing::error!(?e, "MpscCommandSender failed to send command");
        }
    }
}

impl<
    R: crate::features::module_runtime::ports::ModuleRegistryPort<F> + 'static,
    F: crate::shared::rendering::ports::canvas::CanvasFactory + 'static,
> CrankyApp<R, F>
{
    /// Creates a new [`CrankyApp`] instance and initializes modules from config.
    ///
    /// # Errors
    ///
    /// Returns [`AppError::Module`] if initial module loading fails.
    #[allow(clippy::needless_pass_by_value)]
    pub fn new(
        hub: Arc<SignalHub>,
        config: Config,
        command_rx: mpsc::Receiver<AppCommand>,
        command_tx: mpsc::Sender<AppCommand>,
        surface_manager: DynSurfaceManager,
        canvas_factory: Arc<std::sync::Mutex<F>>,
        mut registry: Box<R>,
    ) -> Result<Self, AppError> {
        registry.load(&config).map_err(AppError::Module)?;

        let root_module = registry.root_module();
        let module_ids = registry.module_ids().to_vec();
        let module_names = registry.module_names().clone();
        let name_to_ids = registry.name_to_ids().clone();
        let command_tx_arc = Arc::new(MpscCommandSender(command_tx.clone()));
        let layout_senders = registry.spawn_all(
            hub.clone(),
            surface_manager.clone(),
            command_tx_arc,
            canvas_factory.clone(),
        );

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
            hub,
            read_model,
            command_rx,
            layout_senders,
            surface_manager: surface_manager.clone(),
            command_tx_clone: command_tx,
            registry,
            canvas_factory,
        })
    }

    #[must_use]
    pub fn active_signals(
        &self,
    ) -> &std::collections::HashSet<crate::shared::events::signals::SignalKind> {
        self.registry.active_signal_subscriptions()
    }

    /// Runs the main event loop, listening for display events, commands, and signals.
    ///
    /// # Errors
    ///
    /// Returns [`AppError::Internal`] if display server communication or event dispatching fails.
    #[allow(clippy::too_many_lines)]
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
                Some(mut command) = self.command_rx.recv() => {
                    let mut needs_render = false;
                    let mut process_count: usize = 0;
                    loop {
                        process_count = process_count.saturating_add(1);
                        match command {
                            AppCommand::ContainerLayoutsCalculated {
                                parent_id: _,
                                monitor_id,
                                layouts,
                            } => {
                                for child_layout in layouts {
                                    if let Some(ids) =
                                        self.read_model.name_to_ids.get(child_layout.key().name())
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
                                needs_render = true;
                            }
                            AppCommand::ChildModuleSizeChanged {
                                parent_id: _,
                                child_key,
                                monitor_id,
                                size,
                            } => {
                                let mut sizes_map = self.hub.module_sizes_rx().borrow().clone();
                                let mon_entry = sizes_map.entry(monitor_id.clone()).or_default();
                                mon_entry.insert(child_key, size);
                                let _ = self.hub.module_sizes_tx().send(sizes_map);
                                needs_render = true;
                            }
                            AppCommand::RequestRender => {
                                needs_render = true;
                            },
                            AppCommand::Exec(cmd) => {
                                tracing::debug!("Executing shell command: {cmd}");
                                let _ = std::process::Command::new("sh").arg("-c").arg(cmd).spawn();
                            },
                            AppCommand::SystrayAction { id, action, pos } => {
                                tracing::debug!(?id, ?action, ?pos, "Received AppCommand::SystrayAction, triggering SNI action");
                                match sni.trigger_action(&id, &action, pos).await {
                                    Ok(()) => tracing::debug!(?id, ?action, "SNI trigger_action succeeded"),
                                    Err(e) => tracing::error!(?id, ?action, err = ?e, "SNI trigger_action failed"),
                                }
                            }
                            AppCommand::ModuleSizeChanged(monitor_id, module_id, size) => {
                                self.handle_size_changed(monitor_id, module_id, size);
                                needs_render = true;
                            }
                            AppCommand::ShowTooltip { layout } => {
                                tracing::debug!(?layout, "Received AppCommand::ShowTooltip, calling display.show_tooltip");
                                match display.show_tooltip(*layout) {
                                    Ok(()) => tracing::debug!("display.show_tooltip succeeded"),
                                    Err(e) => tracing::error!(err = ?e, "display.show_tooltip failed"),
                                }
                            }
                            AppCommand::HideTooltip => {
                                tracing::debug!("Received AppCommand::HideTooltip, calling display.hide_tooltip");
                                match display.hide_tooltip() {
                                    Ok(()) => tracing::debug!("display.hide_tooltip succeeded"),
                                    Err(e) => tracing::error!(err = ?e, "display.hide_tooltip failed"),
                                }
                            }
                            AppCommand::ReloadModule(name) => {
                                tracing::info!("Reloading module: {name}");
                                match self.registry.reload_module(&name, &self.read_model.config, self.hub.clone(), self.surface_manager.clone(), Arc::new(MpscCommandSender(self.command_tx_clone.clone())), self.canvas_factory.clone()) {
                                    Ok(new_senders) => {
                                        for (id, sender) in new_senders {
                                            self.layout_senders.insert(id, sender);
                                        }
                                        needs_render = true;
                                    }
                                    Err(e) => tracing::error!("Failed to reload module {name}: {e}"),
                                }
                            }
                            AppCommand::ReloadStyle(sheet_name) => {
                                tracing::info!("Reloading style: {sheet_name}");
                                if sheet_name.as_str() == "base" {
                                    tracing::debug!("Base stylesheet changed; reloading all active modules");
                                    let all_modules: Vec<_> = self.read_model.module_names.values().cloned().collect();
                                    for mod_name in all_modules {
                                        if let Ok(new_senders) = self.registry.reload_module(&mod_name, &self.read_model.config, self.hub.clone(), self.surface_manager.clone(), Arc::new(MpscCommandSender(self.command_tx_clone.clone())), self.canvas_factory.clone()) {
                                            for (id, sender) in new_senders {
                                                self.layout_senders.insert(id, sender);
                                            }
                                        }
                                    }
                                    needs_render = true;
                                } else {
                                    let mods = self.registry.modules_using_style(&sheet_name);
                                    tracing::debug!(
                                        stylesheet = %sheet_name.as_str(),
                                        dependent_modules = ?mods.iter().map(super::super::shared::primitives::ModuleName::as_str).collect::<Vec<_>>(),
                                        "Reloading modules dependent on modified stylesheet"
                                    );
                                    for mod_name in mods {
                                        match self.registry.reload_module(&mod_name, &self.read_model.config, self.hub.clone(), self.surface_manager.clone(), Arc::new(MpscCommandSender(self.command_tx_clone.clone())), self.canvas_factory.clone()) {
                                            Ok(new_senders) => {
                                                for (id, sender) in new_senders {
                                                    self.layout_senders.insert(id, sender);
                                                }
                                                needs_render = true;
                                            }
                                            Err(e) => tracing::error!("Failed to reload module {mod_name}: {e}"),
                                        }
                                    }
                                }
                            }
                            AppCommand::ScriptCall(_) => {
                                // ScriptCall is handled locally by ModuleActor — should not reach here
                                tracing::warn!("Received ScriptCall at application state level; ignoring");
                            }
                        }

                        if process_count > 50 {
                            break;
                        }

                        if let Ok(next_cmd) = self.command_rx.try_recv() {
                            command = next_cmd;
                        } else {
                            break;
                        }
                    }

                    if needs_render {
                        let _ = display.render_all(&self.read_model, &self.layout_senders);
                    }
                }
                Ok(()) = config_rx.changed() => {
                    info!("Config hot-reload triggered in App");
                    let new_config = config_rx.borrow().clone();
                    self.read_model.config = new_config;
                    self.read_model.module_sizes.clear();

                    self.registry.clear();
                    if let Err(e) = self.registry.load(&self.read_model.config) {
                        error!("Failed to reload registry on config change: {e}");
                    } else {
                        self.read_model.root_module = self.registry.root_module();
                        self.read_model.module_ids = self.registry.module_ids().to_vec();
                        self.read_model.module_names.clone_from(self.registry.module_names());
                        self.read_model.name_to_ids.clone_from(self.registry.name_to_ids());
                        self.layout_senders = self.registry.spawn_all(
                            self.hub.clone(),
                            self.surface_manager.clone(),
                            Arc::new(MpscCommandSender(self.command_tx_clone.clone())),
                            self.canvas_factory.clone()
                        );
                    }
                }
                Ok(()) = hyprland_rx.changed() => {
                    let state = hyprland_rx.borrow().clone();
                    let new_focused = state.focused_monitor()
                        .map(|n| n.as_str().to_string())
                        .unwrap_or_default();

                    if new_focused != current_focused_monitor {
                        current_focused_monitor = new_focused;
                        let _ = display.render_all(&self.read_model, &self.layout_senders);
                    }
                }
            }
        }
    }

    #[allow(clippy::needless_pass_by_value)]
    pub fn handle_size_changed(&mut self, monitor_id: MonitorId, module_id: ModuleId, size: Size) {
        let name = self.read_model.module_names.get(&module_id).cloned();
        tracing::trace!(monitor = %monitor_id, module = %module_id, ?size, ?name, "handle_size_changed called");
        self.read_model
            .module_sizes
            .entry(monitor_id.clone())
            .or_default()
            .insert(module_id, size);

        if let Some(name) = name {
            let mut sizes_map = self.hub.module_sizes_rx().borrow().clone();
            let mon_entry = sizes_map.entry(monitor_id).or_default();
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
    use crate::shared::wayland::ports::MockDisplayServerPort;
    use crate::shared::wayland::ports::MockSurfaceManagerPort;
    use std::sync::Arc;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn test_app_initialization() {
        let config = Config::default();
        let hub = Arc::new(SignalHub::new(config.clone()));
        let (_, command_rx) = mpsc::channel(32);
        let (command_tx, _) = mpsc::channel(32);

        let surface_manager: DynSurfaceManager = Arc::new(MockSurfaceManagerPort::new());

        let mut mock_registry = MockModuleRegistryPort::<
            crate::shared::rendering::adapters::tiny_skia::TinySkiaCanvasFactory,
        >::new();
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
            .returning(|_, _, _, _| HashMap::new());

        let canvas_factory = Arc::new(std::sync::Mutex::new(
            crate::shared::rendering::adapters::tiny_skia::TinySkiaCanvasFactory::new(),
        ));

        let app_result = CrankyApp::new(
            hub,
            config,
            command_rx,
            command_tx,
            surface_manager,
            canvas_factory,
            Box::new(mock_registry),
        );

        assert!(app_result.is_ok());
    }

    #[tokio::test]
    async fn test_app_run_exit_on_display_error() {
        let config = Config::default();
        let hub = Arc::new(SignalHub::new(config.clone()));
        let (command_tx, command_rx) = mpsc::channel(32);

        let surface_manager: DynSurfaceManager = Arc::new(MockSurfaceManagerPort::new());

        let mut mock_registry = MockModuleRegistryPort::<
            crate::shared::rendering::adapters::tiny_skia::TinySkiaCanvasFactory,
        >::new();
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
            .returning(|_, _, _, _| HashMap::new());
        mock_registry
            .expect_register_dbus_subscriptions()
            .returning(|_| Box::pin(std::future::ready(())));
        mock_registry.expect_clear().returning(|| ());

        let canvas_factory = Arc::new(std::sync::Mutex::new(
            crate::shared::rendering::adapters::tiny_skia::TinySkiaCanvasFactory::new(),
        ));

        let mut app = CrankyApp::new(
            hub.clone(),
            config,
            command_rx,
            command_tx.clone(),
            surface_manager,
            canvas_factory,
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
                crate::shared::primitives::DynamicValue::String("hour".to_string()),
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
        );

        let mut name_to_ids = HashMap::new();
        name_to_ids.insert(
            crate::shared::primitives::ModuleName::new("hour"),
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
                    crate::shared::primitives::ModuleName::new("hour"),
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

    #[tokio::test]
    async fn test_app_run_commands_and_signals() {
        let config = Config::default();
        let hub = Arc::new(SignalHub::new(config.clone()));
        let (command_tx, command_rx) = mpsc::channel(32);

        let surface_manager: DynSurfaceManager = Arc::new(MockSurfaceManagerPort::new());

        let mut mock_registry = MockModuleRegistryPort::<
            crate::shared::rendering::adapters::tiny_skia::TinySkiaCanvasFactory,
        >::new();
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
            .returning(|_, _, _, _| HashMap::new());
        mock_registry
            .expect_register_dbus_subscriptions()
            .returning(|_| Box::pin(std::future::ready(())));
        mock_registry.expect_clear().returning(|| ());

        let canvas_factory = Arc::new(std::sync::Mutex::new(
            crate::shared::rendering::adapters::tiny_skia::TinySkiaCanvasFactory::new(),
        ));

        let mut app = CrankyApp::new(
            hub.clone(),
            config,
            command_rx,
            command_tx.clone(),
            surface_manager,
            canvas_factory,
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
        mock_display.expect_show_tooltip().returning(|_| Ok(()));
        mock_display.expect_hide_tooltip().returning(|| Ok(()));

        let mock_conn = crate::shared::dbus::ports::MockDbusConnectionPort::new();
        let mock_dbus = crate::shared::dbus::subscription_manager::DbusSubscriptionManager::new(
            std::sync::Arc::new(mock_conn),
            &hub,
        );
        let mut mock_sni = crate::features::systray::ports::MockSniPort::new();
        mock_sni.expect_trigger_action().returning(|_, _, _| Ok(()));

        // Queue commands
        command_tx.send(AppCommand::RequestRender).await.unwrap();
        command_tx
            .send(AppCommand::ModuleSizeChanged(
                MonitorId::new("1"),
                ModuleId::new(1),
                Size::new(10, 10),
            ))
            .await
            .unwrap();
        command_tx
            .send(AppCommand::ShowTooltip {
                layout: Box::new(crate::features::layout_engine::domain::StyledNode::Text {
                    text: crate::features::layout_engine::domain::TextContent::new("t".to_string()),
                    style: crate::features::styling::domain::ComputedStyle::default(),
                    on_click: None,
                    on_hover: None,
                    tooltip: None,
                }),
            })
            .await
            .unwrap();
        command_tx.send(AppCommand::HideTooltip).await.unwrap();
        command_tx
            .send(AppCommand::SystrayAction {
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
        let (command_tx, command_rx) = mpsc::channel(32);
        let surface_manager: DynSurfaceManager = Arc::new(MockSurfaceManagerPort::new());

        let mut mock_registry = MockModuleRegistryPort::<
            crate::shared::rendering::adapters::tiny_skia::TinySkiaCanvasFactory,
        >::new();
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
            crate::shared::primitives::ModuleName::new("hour"),
        );
        mock_registry.expect_module_names().return_const(names);
        let mut name_to_ids = HashMap::new();
        name_to_ids.insert(
            crate::shared::primitives::ModuleName::new("bar"),
            vec![ModuleId::new(0)],
        );
        name_to_ids.insert(
            crate::shared::primitives::ModuleName::new("hour"),
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

        mock_registry
            .expect_spawn_all()
            .return_once(|_, _, _, _| senders);
        mock_registry
            .expect_register_dbus_subscriptions()
            .returning(|_| Box::pin(std::future::ready(())));
        mock_registry.expect_clear().returning(|| ());

        let canvas_factory = Arc::new(std::sync::Mutex::new(
            crate::shared::rendering::adapters::tiny_skia::TinySkiaCanvasFactory::new(),
        ));

        let mut app = CrankyApp::new(
            hub.clone(),
            config,
            command_rx,
            command_tx.clone(),
            surface_manager,
            canvas_factory,
            Box::new(mock_registry),
        )
        .unwrap();

        // 1. Send ContainerLayoutsCalculated for DP-1
        command_tx
            .send(AppCommand::ContainerLayoutsCalculated {
                parent_id: ModuleId::new(0),
                monitor_id: MonitorId::new("DP-1"),
                layouts: vec![crate::shared::primitives::ChildModuleLayout::new(
                    crate::shared::primitives::ModuleKey::from_name(
                        crate::shared::primitives::ModuleName::new("hour"),
                    ),
                    Rect::new(Position::new(100, 0), Size::new(80, 24)),
                )],
            })
            .await
            .unwrap();

        // 2. Send ContainerLayoutsCalculated for DP-2
        command_tx
            .send(AppCommand::ContainerLayoutsCalculated {
                parent_id: ModuleId::new(0),
                monitor_id: MonitorId::new("DP-2"),
                layouts: vec![crate::shared::primitives::ChildModuleLayout::new(
                    crate::shared::primitives::ModuleKey::from_name(
                        crate::shared::primitives::ModuleName::new("hour"),
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
        mock_display.expect_show_tooltip().returning(|_| Ok(()));
        mock_display.expect_hide_tooltip().returning(|| Ok(()));

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
}
