use crate::ports::canvas::Canvas;
use crate::domain::signals::SignalHub;
use crate::domain::errors::DomainError;
use crate::domain::commands::AppCommand;
use crate::modules::ModuleRegistry;
use crate::config::Config;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{info, debug, info_span, debug_span};

pub struct CrankyApp {
    hub: Arc<SignalHub>,
    registry: ModuleRegistry,
    config: Config,
    command_tx: mpsc::Sender<AppCommand>,
    dirty_rx: mpsc::Receiver<u32>,
}

impl CrankyApp {
    pub fn new(
        hub: Arc<SignalHub>,
        dirty_rx: mpsc::Receiver<u32>,
        config: Config,
        command_tx: mpsc::Sender<AppCommand>
    ) -> Self {
        let mut registry = ModuleRegistry::new();
        let _ = registry.load(&config); 
        registry.attach_all(&hub);

        Self {
            hub,
            registry,
            config,
            command_tx,
            dirty_rx,
        }
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    /// The main core loop that awaits reactive signals and issues adapter commands.
    pub async fn run(&mut self) -> Result<(), DomainError> {
        let mut config_rx = self.hub.config_rx();

        info!("Core app loop started, issuing initial render request.");
        let _ = self.command_tx.send(AppCommand::RequestRender(0)).await;

        loop {
            let core_loop_span = info_span!("core_loop_iteration");
            let _enter = core_loop_span.enter();

            tokio::select! {
                Some(target_id) = self.dirty_rx.recv() => {
                    self.handle_dirty(target_id).await?;
                }
                Ok(_) = config_rx.changed() => {
                    let new_config = config_rx.borrow().clone();
                    self.handle_config_change(new_config).await?;
                }
            }
        }
    }

    async fn handle_dirty(&mut self, target_id: u32) -> Result<(), DomainError> {
        let span = debug_span!("handle_dirty", target_id);
        let _enter = span.enter();
        debug!("Module {} signaled dirty, requesting render.", target_id);
        // Broad render request for now. Phase 4 will refine this to specific outputs.
        let _ = self.command_tx.send(AppCommand::RequestRender(0)).await;
        Ok(())
    }

    async fn handle_config_change(&mut self, config: Config) -> Result<(), DomainError> {
        let span = info_span!("handle_config_change");
        let _enter = span.enter();
        info!("Config change detected in core app.");
        self.config = config;
        self.registry.load(&self.config)?;
        self.registry.attach_all(&self.hub);
        let _ = self.command_tx.send(AppCommand::RequestRender(0)).await;
        Ok(())
    }

    /// Renders the current state of modules for a specific monitor onto the provided canvas.
    /// This is called by the adapter in response to a RequestRender command.
    pub fn render(&mut self, _output_id: u32, canvas: &mut dyn Canvas, monitor: &str) -> Result<(), DomainError> {
        // Synchronize all modules with the latest signal data before viewing
        self.registry.refresh_all(&self.hub);

        // Render left modules
        for module in self.registry.left_modules() {
            module.view(canvas, monitor);
        }

        // Render center modules
        for module in self.registry.center_modules() {
            module.view(canvas, monitor);
        }

        // Render right modules
        for module in self.registry.right_modules() {
            module.view(canvas, monitor);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::canvas::MockCanvas;

    #[tokio::test]
    async fn test_app_handle_dirty() {
        let (hub, dirty_rx) = SignalHub::new(Config::default());
        let (command_tx, mut command_rx) = mpsc::channel(10);
        
        let mut app = CrankyApp::new(Arc::new(hub), dirty_rx, Config::default(), command_tx);

        app.handle_dirty(0).await.unwrap();
        
        let cmd = command_rx.recv().await.unwrap();
        match cmd {
            AppCommand::RequestRender(id) => assert_eq!(id, 0),
            _ => panic!("Expected RequestRender command"),
        }
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
