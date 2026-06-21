use crate::domain::commands::AppCommand;
use crate::domain::config::Config;
use crate::domain::signals::SignalHub;
use crate::domain::{
    ModuleId, MonitorId,
    shared::geometry::{BarWidth, Position, Rect, Size},
};
use crate::ports::DisplayServerPort;
use crate::ports::registry::ModuleRegistryPort;
use crate::ports::surface::DynSurfaceManager;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{error, info};

#[derive(Debug)]
pub enum AppError {
    Module(String),
    Internal { message: String },
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Module(msg) => write!(f, "Module error: {}", msg)?,
            Self::Internal { message } => write!(f, "Internal error: {}", message)?,
        }
        Ok(())
    }
}

impl std::error::Error for AppError {}

pub struct ModuleLayout {
    id: ModuleId,
    bounds: Rect,
}

impl ModuleLayout {
    pub fn id(&self) -> crate::domain::ModuleId {
        self.id
    }

    pub fn bounds(&self) -> &Rect {
        &self.bounds
    }
}

pub struct AppReadModel {
    config: Config,
    left_modules: Vec<ModuleId>,
    center_modules: Vec<ModuleId>,
    right_modules: Vec<ModuleId>,
    module_sizes: HashMap<MonitorId, HashMap<ModuleId, Size>>,
}

impl AppReadModel {
    pub fn config(&self) -> &crate::domain::config::Config {
        &self.config
    }

    pub fn calculate_layout(
        &self,
        monitor: &MonitorId,
        bar_width: BarWidth,
        layout_senders: &HashMap<ModuleId, Box<dyn crate::ports::registry::LayoutSender>>,
        bar_config: &crate::domain::config::BarConfig,
    ) -> Vec<ModuleLayout> {
        let mut layouts = Vec::new();
        let bar_height = bar_config.height();
        let border_size = bar_config.border().size().value();
        let padding = bar_config.padding();

        let inner_left = border_size + padding.left().value() as f32;
        let inner_right = border_size + padding.right().value() as f32;
        let inner_top = border_size + padding.top().value() as f32;
        let inner_bottom = border_size + padding.bottom().value() as f32;

        let available_height = bar_height as f32 - inner_top - inner_bottom;

        let get_size = |id: &ModuleId| {
            self.module_sizes
                .get(monitor)
                .and_then(|m| m.get(id))
                .cloned()
                .unwrap_or(Size::new(0, 0))
        };

        // Calculate left modules
        let mut left_x = inner_left;
        for &id in &self.left_modules {
            let size = get_size(&id);
            let y = inner_top + (available_height - size.height() as f32).max(0.0) / 2.0;
            layouts.push(ModuleLayout {
                id,
                bounds: Rect::new(Position::new(left_x as i32, y as i32), size),
            });
            left_x += size.width() as f32;
        }

        // Calculate right modules
        let mut right_x = bar_width.value() as f32 - inner_right;
        let mut right_layouts = Vec::new();
        for &id in self.right_modules.iter().rev() {
            let size = get_size(&id);
            right_x -= size.width() as f32;
            let y = inner_top + (available_height - size.height() as f32).max(0.0) / 2.0;
            right_layouts.push(ModuleLayout {
                id,
                bounds: Rect::new(Position::new(right_x as i32, y as i32), size),
            });
        }
        layouts.extend(right_layouts.into_iter().rev());

        // Calculate center modules
        let mut center_width = 0.0;
        let mut center_sizes = Vec::new();
        for &id in &self.center_modules {
            let size = get_size(&id);
            center_width += size.width() as f32;
            center_sizes.push((id, size));
        }

        let mut center_x = (bar_width.value() as f32 - center_width) / 2.0;
        for (id, size) in center_sizes {
            let y = inner_top + (available_height - size.height() as f32).max(0.0) / 2.0;
            layouts.push(ModuleLayout {
                id,
                bounds: Rect::new(Position::new(center_x as i32, y as i32), size),
            });
            center_x += size.width() as f32;
        }

        // Broadcast layout bounds to modules for this monitor
        let mut updates_by_module: HashMap<ModuleId, HashMap<MonitorId, Rect>> = HashMap::new();
        for layout in &layouts {
            // Keep existing rects for other monitors
            let mut all_rects = HashMap::new();
            if let Some(sender) = layout_senders.get(&layout.id) {
                all_rects = sender.current_layout();
            }
            all_rects.insert(monitor.clone(), layout.bounds);
            updates_by_module.insert(layout.id, all_rects);
        }

        for (id, rects) in updates_by_module {
            if let Some(sender) = layout_senders.get(&id) {
                sender.send_layout(rects);
            }
        }

        layouts
    }
}

pub struct CrankyApp {
    hub: Arc<SignalHub>,
    read_model: AppReadModel,
    command_rx: mpsc::Receiver<AppCommand>,
    layout_senders: HashMap<ModuleId, Box<dyn crate::ports::registry::LayoutSender>>,
    surface_manager: DynSurfaceManager,
    command_tx_clone: mpsc::Sender<AppCommand>,
    registry: Box<dyn crate::ports::registry::ModuleRegistryPort>,
}

struct MpscCommandSender(mpsc::Sender<AppCommand>);
impl crate::ports::registry::CommandSender for MpscCommandSender {
    fn send_command(&self, cmd: AppCommand) {
        let _ = self.0.try_send(cmd);
    }
}

impl CrankyApp {
    pub fn new<R: ModuleRegistryPort + 'static>(
        hub: Arc<SignalHub>,
        config: Config,
        command_rx: mpsc::Receiver<AppCommand>,
        command_tx: mpsc::Sender<AppCommand>,
        surface_manager: DynSurfaceManager,
        mut registry: Box<R>,
    ) -> Result<Self, AppError> {
        registry.load(&config).map_err(AppError::Module)?;

        let left_modules = registry.left_modules();
        let center_modules = registry.center_modules();
        let right_modules = registry.right_modules();
        let command_tx_arc = Arc::new(MpscCommandSender(command_tx.clone()));
        let layout_senders =
            registry.spawn_all(hub.clone(), surface_manager.clone(), command_tx_arc);

        let read_model = AppReadModel {
            config,
            left_modules,
            center_modules,
            right_modules,
            module_sizes: HashMap::new(),
        };

        Ok(Self {
            hub,
            read_model,
            command_rx,
            layout_senders,
            surface_manager: surface_manager.clone(),
            command_tx_clone: command_tx,
            registry,
        })
    }

    pub async fn run(
        &mut self,
        mut display: impl DisplayServerPort,
        mut dbus: impl crate::ports::DBusPort, // Left here for API compatibility
        sni: impl crate::ports::sni::SniPort,
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
                    let mut process_count = 0;
                    loop {
                        process_count += 1;
                        match command {

                            AppCommand::RequestRender => {
                                // Ignore here, usually handled directly by rendering system or triggers re-render 
                            },
                            AppCommand::AppletAction { id, action } => {
                                let _ = sni.trigger_action(&id, &action).await;
                            }
                            AppCommand::ModuleSizeChanged(monitor_id, module_id, size) => {
                                self.handle_size_changed(monitor_id, module_id, size);
                                needs_render = true;
                            }
                            AppCommand::ShowTooltip { text } => {
                                let _ = display.show_tooltip(&text);
                            }
                            AppCommand::HideTooltip => {
                                let _ = display.hide_tooltip();
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
                Ok(_) = config_rx.changed() => {
                    info!("Config hot-reload triggered in App");
                    let new_config = config_rx.borrow().clone();
                    self.read_model.config = new_config;
                    self.read_model.module_sizes.clear();

                    self.registry.clear();
                    if let Err(e) = self.registry.load(&self.read_model.config) {
                        error!("Failed to reload registry on config change: {}", e);
                    } else {
                        self.read_model.left_modules = self.registry.left_modules();
                        self.read_model.center_modules = self.registry.center_modules();
                        self.read_model.right_modules = self.registry.right_modules();
                        self.layout_senders = self.registry.spawn_all(
                            self.hub.clone(),
                            self.surface_manager.clone(),
                            Arc::new(MpscCommandSender(self.command_tx_clone.clone()))
                        );
                    }
                }
                Ok(_) = hyprland_rx.changed() => {
                    let state = hyprland_rx.borrow().clone();
                    let new_focused = state.monitors().iter()
                        .find(|m| m.focused())
                        .map(|m| m.name().as_str().to_string())
                        .unwrap_or_default();

                    if new_focused != current_focused_monitor {
                        current_focused_monitor = new_focused;
                        let _ = display.render_all(&self.read_model, &self.layout_senders);
                    }
                }
            }
        }
    }

    pub fn handle_size_changed(&mut self, monitor_id: MonitorId, module_id: ModuleId, size: Size) {
        self.read_model
            .module_sizes
            .entry(monitor_id.clone())
            .or_default()
            .insert(module_id, size);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::MockDisplayServerPort;
    use crate::ports::registry::MockModuleRegistryPort;
    use crate::ports::surface::MockSurfaceManagerPort;
    use std::sync::Arc;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn test_app_initialization() {
        let config = Config::default();
        let hub = Arc::new(SignalHub::new(config.clone()));
        let (_, command_rx) = mpsc::channel(32);
        let (command_tx, _) = mpsc::channel(32);

        let surface_manager: DynSurfaceManager = Arc::new(MockSurfaceManagerPort::new());

        let mut mock_registry = MockModuleRegistryPort::new();
        mock_registry.expect_load().returning(|_| Ok(()));
        mock_registry.expect_left_modules().returning(Vec::new);
        mock_registry.expect_center_modules().returning(Vec::new);
        mock_registry.expect_right_modules().returning(Vec::new);
        mock_registry
            .expect_spawn_all()
            .returning(|_, _, _| HashMap::new());

        let app_result = CrankyApp::new(
            hub,
            config,
            command_rx,
            command_tx,
            surface_manager,
            Box::new(mock_registry),
        );

        assert!(app_result.is_ok());
    }

    #[tokio::test]
    async fn test_app_run_exit_on_display_error() {
        let config = Config::default();
        let hub = Arc::new(SignalHub::new(config.clone()));
        let (_, command_rx) = mpsc::channel(32);
        let (command_tx, _) = mpsc::channel(32);

        let surface_manager: DynSurfaceManager = Arc::new(MockSurfaceManagerPort::new());

        let mut mock_registry = MockModuleRegistryPort::new();
        mock_registry.expect_load().returning(|_| Ok(()));
        mock_registry.expect_left_modules().returning(Vec::new);
        mock_registry.expect_center_modules().returning(Vec::new);
        mock_registry.expect_right_modules().returning(Vec::new);
        mock_registry
            .expect_spawn_all()
            .returning(|_, _, _| HashMap::new());
        mock_registry
            .expect_register_dbus_subscriptions()
            .returning(|_| Box::pin(std::future::ready(())));

        let mut app = CrankyApp::new(
            hub,
            config,
            command_rx,
            command_tx,
            surface_manager,
            Box::new(mock_registry),
        )
        .unwrap();

        let mut mock_display = MockDisplayServerPort::new();
        mock_display.expect_flush().returning(|| Ok(()));
        mock_display.expect_wait_for_events().returning(|| {
            Box::pin(std::future::ready(Err(
                crate::ports::DisplayServerError::Internal("Test error".into()),
            )))
        });

        let mock_dbus = crate::ports::dbus::MockDBusPort::new();
        let mock_sni = crate::ports::sni::MockSniPort::new();

        let result = app.run(mock_display, mock_dbus, mock_sni).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_calculate_layout_unfocused() {
        let unfocused = crate::domain::config::PartialBarConfig::new(
            None,
            Some(20),
            None,
            None,
            None,
            None,
            None,
            None,
        );
        let default_config = crate::domain::config::BarConfig::default();
        // We need to inject the unfocused config. Since fields are private, we construct a full BarConfig:
        let bar_config = crate::domain::config::BarConfig::new(
            default_config.background().clone(),
            30,
            default_config.vertical_alignment(),
            default_config.border().clone(),
            default_config.margin().clone(),
            default_config.padding().clone(),
            default_config.font_family().clone(),
            default_config.font_size(),
            Some(unfocused),
        );

        let config = Config::new(
            bar_config.clone(),
            crate::domain::config::ModulesConfig::default(),
            crate::domain::config::RenderingMode::default(),
            crate::domain::metrics::MetricsConfig::default(),
        );

        let read_model = AppReadModel {
            config: config.clone(),
            left_modules: vec![],
            center_modules: vec![crate::domain::ModuleId::new(1)],
            right_modules: vec![],
            module_sizes: {
                let mut m = HashMap::new();
                let mut s = HashMap::new();
                s.insert(
                    crate::domain::ModuleId::new(1),
                    crate::domain::shared::geometry::Size::new(50, 10),
                );
                m.insert(MonitorId::new("DP-1"), s);
                m
            },
        };

        let monitor = MonitorId::new("DP-1");
        let layout_senders = HashMap::new();

        // 1. Calculate with focused config
        let layouts_focused = read_model.calculate_layout(
            &monitor,
            BarWidth::new(1920),
            &layout_senders,
            config.bar(),
        );
        assert_eq!(layouts_focused.len(), 1);
        let layout_focused = &layouts_focused[0];

        // height 30, available height = 30, module height = 10, y should be (30 - 10) / 2 = 10
        assert_eq!(layout_focused.bounds().position().y(), 10);

        // 2. Calculate with unfocused config
        let unfocused_bar = config.bar().as_unfocused();
        let layouts_unfocused = read_model.calculate_layout(
            &monitor,
            BarWidth::new(1920),
            &layout_senders,
            &unfocused_bar,
        );
        assert_eq!(layouts_unfocused.len(), 1);
        let layout_unfocused = &layouts_unfocused[0];

        // height 20, available height = 20, module height = 10, y should be (20 - 10) / 2 = 5
        assert_eq!(layout_unfocused.bounds().position().y(), 5);
    }
}
