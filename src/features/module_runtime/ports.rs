use crate::app::commands::AppCommand;
use crate::shared::config::domain::{Config, ModuleConfig};
use crate::shared::events::signals::{SignalHub, SignalKind};
use crate::shared::primitives::{ModuleId, MonitorId, geometry::Rect};
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
        module_name: crate::shared::primitives::ModuleName,
        #[source]
        source: ModuleInitError,
    },
    #[error("Module not found: {0}")]
    ModuleNotFound(crate::shared::primitives::ModuleName),
    #[error("Unsupported engine '{engine}' for module '{module_name}'")]
    UnsupportedEngine {
        engine: String,
        module_name: crate::shared::primitives::ModuleName,
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
    /// Initialize module with configuration.
    ///
    /// # Errors
    ///
    /// Returns `ModuleInitError` if module initialization fails.
    fn init(&mut self, config: &ModuleConfig, full_config: &Config) -> Result<(), ModuleInitError>;
    fn subscriptions(&self) -> &[SignalKind];
    fn dbus_subscriptions(&self) -> &[crate::shared::dbus::domain::DBusSubscription] {
        &[]
    }
    fn styles(&self) -> &[crate::features::styling::domain::StyleSheetName];
    fn refresh(&mut self, hub: &SignalHub, changed_signals: &[SignalKind]);
    fn render(&self, monitor: &MonitorId) -> crate::features::vdom::domain::VNode;

    /// Invoke a named function on the script. Used by `ScriptCall` click actions.
    /// Returns `Ok(())` if the function exists and ran successfully.
    ///
    /// # Errors
    ///
    /// Returns `ModuleInitError` if the function execution fails.
    fn call_function(
        &mut self,
        name: &crate::shared::primitives::FunctionName,
    ) -> Result<(), ModuleInitError>;
}

#[async_trait]
#[cfg_attr(test, mockall::automock)]
pub trait ModuleRegistryPort<Fact: crate::shared::rendering::ports::canvas::CanvasFactory + 'static>:
    Send + Sync
{
    /// Load module configurations into registry.
    ///
    /// # Errors
    ///
    /// Returns `RegistryLoadError` if module loading fails.
    fn load(&mut self, config: &Config) -> Result<(), RegistryLoadError>;
    fn spawn_all(
        &mut self,
        hub: Arc<SignalHub>,
        surface_manager: DynSurfaceManager,
        command_tx: Arc<dyn CommandSender>,
        canvas_factory: Arc<std::sync::Mutex<Fact>>,
    ) -> std::collections::HashMap<ModuleId, Box<dyn LayoutSender>>;

    /// Reload a specific module by name.
    ///
    /// # Errors
    ///
    /// Returns `RegistryLoadError` if module reload fails.
    fn reload_module(
        &mut self,
        name: &crate::shared::primitives::ModuleName,
        config: &Config,
        hub: Arc<SignalHub>,
        surface_manager: DynSurfaceManager,
        command_tx: Arc<dyn CommandSender>,
        canvas_factory: Arc<std::sync::Mutex<Fact>>,
    ) -> Result<std::collections::HashMap<ModuleId, Box<dyn LayoutSender>>, RegistryLoadError>;

    fn root_module(&self) -> Option<ModuleId>;
    fn module_ids(&self) -> &[ModuleId];
    fn module_names(
        &self,
    ) -> &std::collections::HashMap<ModuleId, crate::shared::primitives::ModuleName>;
    fn name_to_ids(
        &self,
    ) -> &std::collections::HashMap<crate::shared::primitives::ModuleName, Vec<ModuleId>>;

    fn modules_using_style(
        &self,
        sheet: &crate::features::styling::domain::StyleSheetName,
    ) -> Vec<crate::shared::primitives::ModuleName>;

    fn clear(&mut self);

    async fn register_dbus_subscriptions(
        &self,
        dbus: &mut crate::shared::dbus::subscription_manager::DbusSubscriptionManager,
    );

    fn active_signal_subscriptions(&self) -> &std::collections::HashSet<SignalKind>;
}
