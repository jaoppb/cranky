use crate::domain::config::Config;
use crate::domain::signals::SignalHub;
use crate::domain::{ModuleId, MonitorId, geometry::{Size, Rect, Position}};
use crate::ports::registry::{ModuleRegistryPort, ModuleContext};
use crate::ports::DisplayServerPort;
use crate::domain::commands::AppCommand;
use crate::ports::surface::DynSurfaceManager;
use tokio::sync::{watch, mpsc};
use std::sync::Arc;
use std::collections::HashMap;
use tracing::{debug, debug_span, info, info_span};

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
    pub fn id(&self) -> ModuleId {
        self.id
    }

    pub fn bounds(&self) -> &Rect {
        &self.bounds
    }
}

pub struct CrankyApp {
    hub: Arc<SignalHub>,
    config: Config,
    command_rx: mpsc::Receiver<AppCommand>,
    left_modules: Vec<ModuleId>,
    center_modules: Vec<ModuleId>,
    right_modules: Vec<ModuleId>,
    layout_senders: HashMap<ModuleId, watch::Sender<HashMap<MonitorId, Rect>>>,
    module_sizes: HashMap<MonitorId, HashMap<ModuleId, Size>>,
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
        registry.load(&config).map_err(|e| AppError::Module(e))?;
        
        let left_modules = registry.left_modules();
        let center_modules = registry.center_modules();
        let right_modules = registry.right_modules();

        let layout_senders = registry.spawn_all(hub.clone(), surface_manager, command_tx);

        Ok(Self {
            hub,
            config,
            command_rx,
            left_modules,
            center_modules,
            right_modules,
            layout_senders,
            module_sizes: HashMap::new(),
        })
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn hub(&self) -> &Arc<SignalHub> {
        &self.hub
    }

    pub async fn run(
        &mut self,
        mut display: impl DisplayServerPort,
        mut dbus: impl crate::ports::DBusPort, // Left here for API compatibility but might need refactoring later
        sni: impl crate::ports::sni::SniPort,
    ) -> Result<(), AppError> {
        let mut config_rx = self.hub.config_rx();

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
                            AppCommand::RequestRender(_output_id) => {
                                needs_render = true;
                            }
                            AppCommand::Input(_module_id, _event) => {
                                // Input handling routed to module tasks
                            }
                            AppCommand::Log(level, msg) => {
                                match level {
                                    tracing::Level::ERROR => tracing::error!("{}", msg),
                                    tracing::Level::WARN => tracing::warn!("{}", msg),
                                    tracing::Level::INFO => tracing::info!("{}", msg),
                                    tracing::Level::DEBUG => tracing::debug!("{}", msg),
                                    tracing::Level::TRACE => tracing::trace!("{}", msg),
                                }
                            }
                            AppCommand::DBusCall(_, _, _, _, _, _) => {}
                            AppCommand::CreateBar(_, _) => {}
                            AppCommand::DestroyBar(_) => {}
                            AppCommand::AppletAction { .. } => {}
                            AppCommand::ModuleSizeChanged(monitor_id, module_id, size) => {
                                self.handle_size_changed(monitor_id, module_id, size);
                                needs_render = true;
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
                        let _ = display.render_all(self);
                    }
                }
                Ok(_) = config_rx.changed() => {
                    // TODO: Hot reload module spawning
                }
            }
        }
    }

    pub fn handle_size_changed(&mut self, monitor_id: MonitorId, module_id: ModuleId, size: Size) {
        self.module_sizes.entry(monitor_id.clone()).or_default().insert(module_id, size);
    }

    pub fn prepare_render(&mut self) {
        // No-op for now, modules react automatically.
    }

    // CrankyApp no longer takes `&mut dyn Canvas` for measuring. It uses the cached `module_sizes`.
    pub fn calculate_layout(&self, monitor: &MonitorId, bar_width: u32) -> Vec<ModuleLayout> {
        let mut layouts = Vec::new();
        let bar_config = self.config.bar();
        let bar_height = bar_config.height();
        let border_size = bar_config.border().size().value();
        let padding = bar_config.padding();
        
        let inner_left = border_size + padding.left().value() as f32;
        let inner_right = border_size + padding.right().value() as f32;
        let inner_top = border_size + padding.top().value() as f32;
        let inner_bottom = border_size + padding.bottom().value() as f32;

        let available_height = bar_height as f32 - inner_top - inner_bottom;
        
        let get_size = |id: &ModuleId| {
            self.module_sizes.get(monitor).and_then(|m| m.get(id)).cloned().unwrap_or(Size::new(0, 0))
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
        let mut right_x = bar_width as f32 - inner_right;
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

        let mut center_x = (bar_width as f32 - center_width) / 2.0;
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
            if let Some(sender) = self.layout_senders.get(&layout.id) {
                all_rects = sender.borrow().clone();
            }
            all_rects.insert(monitor.clone(), layout.bounds.clone());
            updates_by_module.insert(layout.id, all_rects);
        }
        
        for (id, rects) in updates_by_module {
            if let Some(sender) = self.layout_senders.get(&id) {
                let _ = sender.send(rects);
            }
        }

        layouts
    }
}
