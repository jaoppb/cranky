use crate::domain::commands::AppCommand;
use crate::domain::config::Config;
use crate::domain::signals::SignalHub;
use crate::domain::{
    ModuleId, MonitorId,
    shared::geometry::{BarWidth, Position, Rect, Size},
};
use crate::ports::DisplayServerPort;
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

        let available_height = bar_height.value() as f32 - inner_top - inner_bottom;

        let get_size = |id: &ModuleId| {
            self.module_sizes
                .get(monitor)
                .and_then(|m| m.get(id))
                .cloned()
                .unwrap_or(Size::new(0, 0))
        };

        let gap = self.config.bar().module_gap().value() as f32;

        // Calculate left modules
        let mut left_x = inner_left;
        for &id in &self.left_modules {
            let size = get_size(&id);
            let y = inner_top + (available_height - size.height() as f32).max(0.0) / 2.0;
            layouts.push(ModuleLayout {
                id,
                bounds: Rect::new(Position::new(left_x as i32, y as i32), size),
            });
            left_x += size.width() as f32 + gap;
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
            right_x -= gap;
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
        if !center_sizes.is_empty() {
            center_width += ((center_sizes.len() - 1) as f32) * gap;
        }

        let mut center_x = (bar_width.value() as f32 - center_width) / 2.0;
        for (id, size) in center_sizes {
            let y = inner_top + (available_height - size.height() as f32).max(0.0) / 2.0;
            layouts.push(ModuleLayout {
                id,
                bounds: Rect::new(Position::new(center_x as i32, y as i32), size),
            });
            center_x += size.width() as f32 + gap;
        }

        layouts
    }
}

pub struct CrankyApp<R: crate::ports::registry::ModuleRegistryPort<F> + 'static, F: crate::ports::canvas::CanvasFactory + 'static> {
    hub: Arc<SignalHub>,
    read_model: AppReadModel,
    command_rx: mpsc::Receiver<AppCommand>,
    layout_senders: HashMap<ModuleId, Box<dyn crate::ports::registry::LayoutSender>>,
    surface_manager: DynSurfaceManager,
    command_tx_clone: mpsc::Sender<AppCommand>,
    registry: Box<R>,
    canvas_factory: Arc<std::sync::Mutex<F>>,
}

struct MpscCommandSender(mpsc::Sender<AppCommand>);
impl crate::ports::registry::CommandSender for MpscCommandSender {
    fn send_command(&self, cmd: AppCommand) {
        let _ = self.0.try_send(cmd);
    }
}

impl<R: crate::ports::registry::ModuleRegistryPort<F> + 'static, F: crate::ports::canvas::CanvasFactory + 'static> CrankyApp<R, F> {
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

        let left_modules = registry.left_modules();
        let center_modules = registry.center_modules();
        let right_modules = registry.right_modules();
        let command_tx_arc = Arc::new(MpscCommandSender(command_tx.clone()));
        let layout_senders =
            registry.spawn_all(hub.clone(), surface_manager.clone(), command_tx_arc, canvas_factory.clone());

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
            canvas_factory,
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
                                needs_render = true;
                            },
                            AppCommand::Exec(cmd) => {
                                tracing::debug!("Executing shell command: {}", cmd);
                                let _ = std::process::Command::new("sh").arg("-c").arg(cmd).spawn();
                            },
                            AppCommand::AppletAction { id, action } => {
                                let _ = sni.trigger_action(&id, &action).await;
                            }
                            AppCommand::ModuleSizeChanged(monitor_id, module_id, size) => {
                                self.handle_size_changed(monitor_id, module_id, size);
                                needs_render = true;
                            }
                            AppCommand::ShowTooltip { layout } => {
                                let _ = display.show_tooltip(*layout);
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
                            Arc::new(MpscCommandSender(self.command_tx_clone.clone())),
                            self.canvas_factory.clone()
                        );
                    }
                }
                Ok(_) = hyprland_rx.changed() => {
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

        let mut mock_registry = MockModuleRegistryPort::<crate::adapters::rendering::TinySkiaCanvasFactory>::new();
        mock_registry.expect_load().returning(|_| Ok(()));
        mock_registry.expect_left_modules().returning(Vec::new);
        mock_registry.expect_center_modules().returning(Vec::new);
        mock_registry.expect_right_modules().returning(Vec::new);
        mock_registry
            .expect_spawn_all()
            .returning(|_, _, _, _| HashMap::new());

        let canvas_factory = Arc::new(std::sync::Mutex::new(crate::adapters::rendering::TinySkiaCanvasFactory::new()));

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
        let (_, command_rx) = mpsc::channel(32);
        let (command_tx, _) = mpsc::channel(32);

        let surface_manager: DynSurfaceManager = Arc::new(MockSurfaceManagerPort::new());

        let mut mock_registry = MockModuleRegistryPort::<crate::adapters::rendering::TinySkiaCanvasFactory>::new();
        mock_registry.expect_load().returning(|_| Ok(()));
        mock_registry.expect_left_modules().returning(Vec::new);
        mock_registry.expect_center_modules().returning(Vec::new);
        mock_registry.expect_right_modules().returning(Vec::new);
        mock_registry
            .expect_spawn_all()
            .returning(|_, _, _, _| HashMap::new());
        mock_registry
            .expect_register_dbus_subscriptions()
            .returning(|_| Box::pin(std::future::ready(())));

        let canvas_factory = Arc::new(std::sync::Mutex::new(crate::adapters::rendering::TinySkiaCanvasFactory::new()));

        let mut app = CrankyApp::new(
            hub,
            config,
            command_rx,
            command_tx,
            surface_manager,
            canvas_factory,
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
        let unfocused = crate::domain::config::PartialBarConfig::new(crate::domain::config::CreatePartialBarConfigCommand {
            background: None,
            height: Some(crate::domain::shared::geometry::BarHeight::new(20)),
            vertical_alignment: None,
            border: None,
            margin: None,
            padding: None,
            module_gap: None,
            font_family: None,
            font_size: None,
        });
        let default_config = crate::domain::config::BarConfig::default();
        // We need to inject the unfocused config. Since fields are private, we construct a full BarConfig:
        let bar_config = crate::domain::config::BarConfig::new(crate::domain::config::CreateBarConfigCommand {
            background: default_config.background().clone(),
            height: crate::domain::shared::geometry::BarHeight::new(30),
            vertical_alignment: default_config.vertical_alignment(),
            border: default_config.border().clone(),
            margin: default_config.margin().clone(),
            padding: default_config.padding().clone(),
            module_gap: default_config.module_gap(),
            font_family: default_config.font_family().clone(),
            font_size: default_config.font_size(),
            unfocused: Some(unfocused),
        });

        let config = Config::new(
            bar_config.clone(),
            crate::domain::config::ModulesConfig::default(),
            crate::domain::config::RenderingMode::default(),
            crate::domain::metrics::MetricsConfig::default(),
            crate::domain::config::TooltipConfig::default(),
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

        let monitor_1 = MonitorId::new("DP-1");

        // 1. Calculate with focused config
        let layouts_focused = read_model.calculate_layout(
            &monitor_1,
            BarWidth::new(1920),
            config.bar(),
        );
        assert_eq!(layouts_focused.len(), 1);
        let layout_focused = &layouts_focused[0];

        // height 30, available height = 30, module height = 10, y should be (30 - 10) / 2 = 10
        assert_eq!(layout_focused.bounds().position().y(), 10);

        // 2. Calculate with unfocused config
        let unfocused_bar = config.bar().as_unfocused();
        let layouts_unfocused = read_model.calculate_layout(
            &monitor_1,
            BarWidth::new(1920),
            &unfocused_bar,
        );
        assert_eq!(layouts_unfocused.len(), 1);
        let layout_unfocused = &layouts_unfocused[0];

        // height 20, available height = 20, module height = 10, y should be (20 - 10) / 2 = 5
        assert_eq!(layout_unfocused.bounds().position().y(), 5);
    }

    #[test]
    fn test_app_error_fmt() {
        let err1 = AppError::Module("test".into());
        assert_eq!(err1.to_string(), "Module error: test");
        let err2 = AppError::Internal { message: "test".into() };
        assert_eq!(err2.to_string(), "Internal error: test");
    }

    #[test]
    fn test_calculate_layout_left_right() {
        let config = Config::default();
        let mut read_model = AppReadModel {
            config: config.clone(),
            left_modules: vec![ModuleId::new(1), ModuleId::new(2)],
            center_modules: vec![],
            right_modules: vec![ModuleId::new(3)],
            module_sizes: HashMap::new(),
        };
        
        let mut sizes = HashMap::new();
        sizes.insert(ModuleId::new(1), Size::new(100, 20));
        sizes.insert(ModuleId::new(2), Size::new(50, 20));
        sizes.insert(ModuleId::new(3), Size::new(80, 20));
        read_model.module_sizes.insert(MonitorId::new("DP-1"), sizes);
        
        let layouts = read_model.calculate_layout(&MonitorId::new("DP-1"), BarWidth::new(1920), config.bar());
        assert_eq!(layouts.len(), 3);
        
        let gap = config.bar().module_gap().value() as i32;
        
        let l1 = layouts.iter().find(|l| l.id() == ModuleId::new(1)).unwrap();
        assert_eq!(l1.bounds().x(), 0);
        
        let l2 = layouts.iter().find(|l| l.id() == ModuleId::new(2)).unwrap();
        assert_eq!(l2.bounds().x(), 100 + gap);
        
        let l3 = layouts.iter().find(|l| l.id() == ModuleId::new(3)).unwrap();
        assert_eq!(l3.bounds().x(), 1920 - 80); 
    }

    #[tokio::test]
    async fn test_app_run_commands_and_signals() {
        let config = Config::default();
        let hub = Arc::new(SignalHub::new(config.clone()));
        let (command_tx, command_rx) = mpsc::channel(32);
        
        let surface_manager: DynSurfaceManager = Arc::new(MockSurfaceManagerPort::new());

        let mut mock_registry = MockModuleRegistryPort::<crate::adapters::rendering::TinySkiaCanvasFactory>::new();
        mock_registry.expect_load().returning(|_| Ok(()));
        mock_registry.expect_left_modules().returning(Vec::new);
        mock_registry.expect_center_modules().returning(Vec::new);
        mock_registry.expect_right_modules().returning(Vec::new);
        mock_registry.expect_spawn_all().returning(|_, _, _, _| HashMap::new());
        mock_registry.expect_register_dbus_subscriptions().returning(|_| Box::pin(std::future::ready(())));
        mock_registry.expect_clear().returning(|| ());

        let canvas_factory = Arc::new(std::sync::Mutex::new(crate::adapters::rendering::TinySkiaCanvasFactory::new()));

        let mut app = CrankyApp::new(
            hub.clone(),
            config,
            command_rx,
            command_tx.clone(),
            surface_manager,
            canvas_factory,
            Box::new(mock_registry),
        ).unwrap();

        let mut mock_display = MockDisplayServerPort::new();
        mock_display.expect_flush().returning(|| Ok(()));
        
        // Let it succeed twice, then fail to exit the loop
        let mut call_count = 0;
        mock_display.expect_wait_for_events().returning(move || {
            call_count += 1;
            if call_count <= 2 {
                Box::pin(std::future::ready(Ok(())))
            } else {
                Box::pin(std::future::ready(Err(crate::ports::DisplayServerError::Internal("Exit".into()))))
            }
        });
        mock_display.expect_dispatch_pending().returning(|| Ok(()));
        mock_display.expect_render_all().returning(|_, _| Ok(()));
        mock_display.expect_show_tooltip().returning(|_| Ok(()));
        mock_display.expect_hide_tooltip().returning(|| Ok(()));

        let mock_dbus = crate::ports::dbus::MockDBusPort::new();
        let mut mock_sni = crate::ports::sni::MockSniPort::new();
        mock_sni.expect_trigger_action().returning(|_, _| Ok(()));

        // Queue commands
        command_tx.send(AppCommand::RequestRender).await.unwrap();
        command_tx.send(AppCommand::ModuleSizeChanged(MonitorId::new("1"), ModuleId::new(1), Size::new(10, 10))).await.unwrap();
        command_tx.send(AppCommand::ShowTooltip { layout: Box::new(crate::domain::layout::LayoutNode::Text { text: crate::domain::layout::TextContent::new("t".into()), color: crate::domain::shared::color::DrawingColor::Solid(crate::domain::shared::color::Color::new(0, 0, 0, 255)), font: None, size: None, on_click: None, on_hover: None, tooltip: None }) }).await.unwrap();
        command_tx.send(AppCommand::HideTooltip).await.unwrap();
        command_tx.send(AppCommand::AppletAction { id: "a".into(), action: "b".into() }).await.unwrap();
        
        // Trigger config and hyprland changes
        hub.config_tx().send(Config::default()).unwrap();
        hub.hyprland_tx().send(crate::domain::signals::HyprlandState::new(std::collections::BTreeMap::new(), std::collections::BTreeMap::new(), Some(crate::domain::workspace::MonitorName::new("1")))).unwrap();

        let result = app.run(mock_display, mock_dbus, mock_sni).await;
        assert!(result.is_err());
    }
}
