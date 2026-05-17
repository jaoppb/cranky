use crate::config::Config;
use crate::domain::commands::AppCommand;
use crate::domain::errors::DomainError;
use crate::domain::signals::SignalHub;
use crate::domain::{ModuleId, MonitorId, geometry::{Rect, Position}};
use crate::modules::ModuleRegistry;
use crate::ports::canvas::Canvas;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, debug_span, info, info_span};

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
    registry: ModuleRegistry,
    config: Config,
    dirty_rx: mpsc::Receiver<ModuleId>,
}

impl CrankyApp {
    pub fn new(
        hub: Arc<SignalHub>,
        dirty_rx: mpsc::Receiver<ModuleId>,
        config: Config,
        _command_tx: mpsc::Sender<AppCommand>,
    ) -> Self {
        let mut registry = ModuleRegistry::new();
        let _ = registry.load(&config);
        registry.attach_all(&hub);

        Self {
            hub,
            registry,
            config,
            dirty_rx,
        }
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn dirty_rx(&mut self) -> &mut mpsc::Receiver<ModuleId> {
        &mut self.dirty_rx
    }

    pub fn hub(&self) -> &Arc<SignalHub> {
        &self.hub
    }

    pub async fn run<D: crate::ports::DisplayServerPort>(
        &mut self,
        display: &mut D,
    ) -> Result<(), DomainError> {
        let mut config_rx = self.hub.config_rx();

        info!("Core app loop started, issuing initial render pass.");
        display
            .render_all(self)
            .map_err(|e| DomainError::Internal {
                message: e.to_string(),
            })?;

        loop {
            let _ = display.flush();

            tokio::select! {
                res = display.wait_for_events() => {
                    res.map_err(|e| DomainError::Internal { message: e.to_string() })?;
                    display.dispatch_pending().map_err(|e| DomainError::Internal { message: e.to_string() })?;
                }
                Some(target_id) = self.dirty_rx.recv() => {
                    self.handle_dirty(target_id).await?;
                    display.render_all(self).map_err(|e| DomainError::Internal { message: e.to_string() })?;
                }
                Ok(_) = config_rx.changed() => {
                    let new_config = config_rx.borrow().clone();
                    self.handle_config_change(new_config).await?;
                    display.render_all(self).map_err(|e| DomainError::Internal { message: e.to_string() })?;
                }
            }
        }
    }

    pub async fn handle_dirty(&mut self, target_id: ModuleId) -> Result<(), DomainError> {
        let span = debug_span!("handle_dirty", target_id = %target_id);
        let _enter = span.enter();
        debug!("Module {} signaled dirty.", target_id);
        Ok(())
    }

    pub async fn handle_config_change(&mut self, config: Config) -> Result<(), DomainError> {
        let span = info_span!("handle_config_change");
        let _enter = span.enter();
        info!("Config change detected in core app.");
        self.config = config;
        self.registry.load(&self.config)?;
        self.registry.attach_all(&self.hub);
        Ok(())
    }

    pub fn prepare_render(&mut self) {
        self.registry.refresh_all(&self.hub);
    }

    pub fn calculate_layout(&self, monitor: &MonitorId, bar_width: u32, canvas: &mut dyn Canvas) -> Vec<ModuleLayout> {
        let mut layouts = Vec::new();
        let bar_height = self.config.bar().height();

        // Calculate left modules
        let mut left_x = 0.0;
        for id in self.registry.left_modules() {
            if let Some(module) = self.registry.get(id) {
                let size = module.measure(canvas, monitor);
                let y = (bar_height.saturating_sub(size.height())) / 2;
                layouts.push(ModuleLayout {
                    id,
                    bounds: Rect::new(Position::new(left_x as i32, y as i32), size),
                });
                left_x += size.width() as f32;
            }
        }

        // Calculate right modules
        let mut right_x = bar_width as f32;
        let mut right_layouts = Vec::new();
        for id in self.registry.right_modules().iter().rev() {
            if let Some(module) = self.registry.get(*id) {
                let size = module.measure(canvas, monitor);
                right_x -= size.width() as f32;
                let y = (bar_height.saturating_sub(size.height())) / 2;
                right_layouts.push(ModuleLayout {
                    id: *id,
                    bounds: Rect::new(Position::new(right_x as i32, y as i32), size),
                });
            }
        }
        layouts.extend(right_layouts.into_iter().rev());

        // Calculate center modules
        let mut center_width = 0.0;
        let mut center_sizes = Vec::new();
        for id in self.registry.center_modules() {
            if let Some(module) = self.registry.get(id) {
                let size = module.measure(canvas, monitor);
                center_width += size.width() as f32;
                center_sizes.push((id, size));
            }
        }

        let mut center_x = (bar_width as f32 - center_width) / 2.0;
        for (id, size) in center_sizes {
            let y = (bar_height.saturating_sub(size.height())) / 2;
            layouts.push(ModuleLayout {
                id,
                bounds: Rect::new(Position::new(center_x as i32, y as i32), size),
            });
            center_x += size.width() as f32;
        }

        layouts
    }

    pub fn render_module(&self, id: ModuleId, canvas: &mut dyn Canvas, monitor: &MonitorId) {
        if let Some(module) = self.registry.get(id) {
            module.view(canvas, monitor);
        }
    }

    /// Renders the current state of modules for a specific monitor onto the provided canvas.
    /// This is called by the adapter in response to a RequestRender command.
    pub fn render(
        &mut self,
        _output_id: u32,
        canvas: &mut dyn Canvas,
        monitor: &str,
    ) -> Result<(), DomainError> {
        let monitor_id = MonitorId::new(monitor);
        self.prepare_render();
        
        let layout = self.calculate_layout(&monitor_id, 1920, canvas); 
        for l in layout {
            self.render_module(l.id(), canvas, &monitor_id);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::errors::PortError;
    use crate::ports::DisplayServerPort;
    use crate::ports::canvas::MockCanvas;
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicU32, Ordering};

    struct MockDisplayServer {
        render_count: Arc<AtomicU32>,
        event_tx: mpsc::Sender<()>,
        event_rx: mpsc::Receiver<()>,
    }

    impl MockDisplayServer {
        fn new() -> (Self, mpsc::Sender<()>) {
            let (event_tx, event_rx) = mpsc::channel(1);
            (
                Self {
                    render_count: Arc::new(AtomicU32::new(0)),
                    event_tx: event_tx.clone(),
                    event_rx,
                },
                event_tx,
            )
        }
    }

    #[async_trait]
    impl DisplayServerPort for MockDisplayServer {
        fn create_bar(&self, _id: u32, _name: &str) -> Result<(), PortError> {
            Ok(())
        }
        fn destroy_bar(&self, _id: u32) -> Result<(), PortError> {
            Ok(())
        }
        async fn wait_for_events(&mut self) -> Result<(), PortError> {
            self.event_rx.recv().await;
            Ok(())
        }
        fn dispatch_pending(&mut self) -> Result<(), PortError> {
            Ok(())
        }
        fn flush(&mut self) -> Result<(), PortError> {
            Ok(())
        }
        fn render_all(&mut self, _app: &mut CrankyApp) -> Result<(), PortError> {
            self.render_count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_app_run_initial_render() {
        let (hub, dirty_rx) = SignalHub::new(Config::default());
        let (command_tx, _) = mpsc::channel(10);
        let mut app = CrankyApp::new(Arc::new(hub), dirty_rx, Config::default(), command_tx);
        let (mut display, _event_trigger) = MockDisplayServer::new();
        let render_count = display.render_count.clone();

        // Run in a task so we can stop it
        let handle = tokio::spawn(async move { app.run(&mut display).await });

        // Give it a moment to perform initial render
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        assert_eq!(render_count.load(Ordering::SeqCst), 1);
        handle.abort();
    }

    #[tokio::test]
    async fn test_app_run_reacts_to_dirty() {
        let (hub, dirty_rx) = SignalHub::new(Config::default());
        let (command_tx, _) = mpsc::channel(10);
        let hub = Arc::new(hub);
        let mut app = CrankyApp::new(hub.clone(), dirty_rx, Config::default(), command_tx);
        let (mut display, _event_trigger) = MockDisplayServer::new();
        let render_count = display.render_count.clone();
        let dirty_tx = hub.dirty_tx();

        let handle = tokio::spawn(async move { app.run(&mut display).await });

        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        assert_eq!(render_count.load(Ordering::SeqCst), 1);

        // Signal dirty
        dirty_tx.send(ModuleId::new(0)).await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;

        // Should have rendered again (initial + dirty)
        assert_eq!(render_count.load(Ordering::SeqCst), 2);
        handle.abort();
    }

    #[test]
    fn test_app_render_calls_modules() {
        let (hub, dirty_rx) = SignalHub::new(Config::default());
        let (command_tx, _) = mpsc::channel(10);
        let mut app = CrankyApp::new(Arc::new(hub), dirty_rx, Config::default(), command_tx);

        let mut mock = MockCanvas::new();
        // Since we have no modules in default config, no calls expected yet.
        // But the method should execute.
        app.render(0, &mut mock, "eDP-1").unwrap();
    }
}
