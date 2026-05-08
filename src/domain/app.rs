use crate::ports::canvas::Canvas;
use crate::domain::signals::SignalHub;
use crate::domain::errors::DomainError;
use crate::domain::commands::AppCommand;
use crate::modules::ModuleRegistry;
use crate::config::Config;
use std::sync::Arc;
use tokio::sync::mpsc;

pub struct CrankyApp<C: Canvas> {
    hub: Arc<SignalHub>,
    registry: ModuleRegistry<C>,
    config: Config,
    command_tx: mpsc::Sender<AppCommand>,
    dirty_rx: mpsc::Receiver<u32>,
}

impl<C: Canvas + 'static> CrankyApp<C> {
    pub fn new(
        hub: Arc<SignalHub>,
        dirty_rx: mpsc::Receiver<u32>,
        config: Config,
        command_tx: mpsc::Sender<AppCommand>
    ) -> Self {
        let mut registry = ModuleRegistry::new();
        // Error handling for registry load will be refined as we move forward
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

    /// The main core loop that awaits reactive signals and issues adapter commands.
    pub async fn run(&mut self) -> Result<(), DomainError> {
        let mut config_rx = self.hub.config_rx();

        loop {
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

    async fn handle_dirty(&mut self, _target_id: u32) -> Result<(), DomainError> {
        // Broad render request for now. Phase 4 will refine this to specific outputs.
        let _ = self.command_tx.send(AppCommand::RequestRender(0)).await;
        Ok(())
    }

    async fn handle_config_change(&mut self, config: Config) -> Result<(), DomainError> {
        self.config = config;
        self.registry.load(&self.config)?;
        self.registry.attach_all(&self.hub);
        let _ = self.command_tx.send(AppCommand::RequestRender(0)).await;
        Ok(())
    }

    /// Renders the current state of modules for a specific monitor onto the provided canvas.
    /// This is called by the adapter in response to a RequestRender command.
    pub fn render(&mut self, _output_id: u32, canvas: &mut C, monitor: &str) -> Result<(), DomainError> {
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
