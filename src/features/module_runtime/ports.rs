use crate::app::commands::AppCommand;
use crate::shared::config::domain::{Config, ModuleConfig};
use crate::shared::events::signals::{SignalHub, SignalKind};
use crate::shared::primitives::{
    ModuleId, MonitorId,
    geometry::Rect,
};
use crate::shared::wayland::ports::DynSurfaceManager;
use async_trait::async_trait;
use std::sync::Arc;
use thiserror::Error;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum ModuleInitError {
    #[error("Script evaluation error: {0}")]
    ScriptError(String),
    #[error("Configuration error: {0}")]
    ConfigError(String),
    #[error("Internal module error: {0}")]
    Internal(String),
}

#[derive(Error, Debug, PartialEq, Eq)]
pub enum RegistryLoadError {
    #[error("Failed to initialize module '{module_name}': {source}")]
    ModuleInit {
        module_name: String,
        #[source] source: ModuleInitError,
    },
    #[error("Module not found: {0}")]
    ModuleNotFound(String),
    #[error("Unsupported engine '{engine}' for module '{module_name}'")]
    UnsupportedEngine {
        engine: String,
        module_name: String,
    },
    #[error("Internal registry error: {0}")]
    Internal(String),
}

pub trait CommandSender: Send + Sync {
    fn send_command(&self, cmd: AppCommand);
}

pub trait LayoutSender: Send + Sync {
    fn send_layout(&self, layout: std::collections::HashMap<MonitorId, Rect>);
}
#[async_trait]
pub trait AnyModulePort: Send + Sync {
    fn init(&mut self, config: &ModuleConfig, full_config: &Config) -> Result<(), ModuleInitError>;
    fn subscriptions(&self) -> &[SignalKind];
    fn refresh(&mut self, hub: &SignalHub, changed_signals: &[SignalKind]);
    fn render(&self, monitor: &MonitorId) -> crate::features::layout_engine::domain::LayoutNode;
}

#[async_trait]
#[cfg_attr(test, mockall::automock)]
pub trait ModuleRegistryPort<Fact: crate::shared::rendering::ports::canvas::CanvasFactory + 'static>: Send + Sync {
    fn load(&mut self, config: &Config) -> Result<(), RegistryLoadError>;
    fn spawn_all(
        &mut self,
        hub: Arc<SignalHub>,
        surface_manager: DynSurfaceManager,
        command_tx: Arc<dyn CommandSender>,
        canvas_factory: Arc<std::sync::Mutex<Fact>>,
    ) -> std::collections::HashMap<ModuleId, Box<dyn LayoutSender>>;

    fn left_modules(&self) -> &[ModuleId];
    fn center_modules(&self) -> &[ModuleId];
    fn right_modules(&self) -> &[ModuleId];

    fn clear(&mut self);

    async fn register_dbus_subscriptions(&self, dbus: &mut dyn crate::shared::dbus::ports::DBusPort);
}
