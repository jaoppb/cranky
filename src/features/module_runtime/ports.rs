use crate::features::layout_engine::domain::DisplayCommandSender;
use crate::features::vdom::domain::UiCommandSender;
use crate::shared::config::domain::{Config, ModuleConfig};
use crate::shared::events::signals::{SignalHub, SignalKind};
use crate::shared::primitives::geometry::{Rect, Size};
use crate::shared::primitives::{ChildModuleLayout, ModuleId, ModuleKey, MonitorId};
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayoutEvent {
    ModuleSizeChanged {
        monitor_id: MonitorId,
        module_id: ModuleId,
        size: Size,
    },
    ChildModuleSizeChanged {
        parent_id: ModuleId,
        child_key: ModuleKey,
        monitor_id: MonitorId,
        size: Size,
    },
    ContainerLayoutsCalculated {
        parent_id: ModuleId,
        monitor_id: MonitorId,
        layouts: Vec<ChildModuleLayout>,
    },
}

pub trait LayoutEventSender: Send + Sync {
    fn send_layout_event(&self, event: LayoutEvent);
}

impl<F> LayoutEventSender for F
where
    F: Fn(LayoutEvent) + Send + Sync,
{
    fn send_layout_event(&self, event: LayoutEvent) {
        self(event);
    }
}

impl LayoutEventSender for tokio::sync::mpsc::Sender<LayoutEvent> {
    fn send_layout_event(&self, event: LayoutEvent) {
        if let Err(e) = self.try_send(event) {
            tracing::error!(?e, "failed to send layout event via tokio channel");
        }
    }
}

impl LayoutEventSender for std::sync::mpsc::Sender<LayoutEvent> {
    fn send_layout_event(&self, event: LayoutEvent) {
        if let Err(e) = self.send(event) {
            tracing::error!(?e, "failed to send layout event via std channel");
        }
    }
}

pub trait LayoutSender: Send + Sync {
    fn send_layout(&self, layout: std::collections::HashMap<MonitorId, Rect>);
}

#[derive(Clone)]
pub struct ModuleRuntimeDependencies<
    Fact: crate::shared::rendering::ports::canvas::CanvasFactory + 'static,
    LS: LayoutEventSender + 'static,
    DS: DisplayCommandSender + 'static,
    US: UiCommandSender + 'static,
> {
    pub hub: Arc<SignalHub>,
    pub surface_manager: DynSurfaceManager,
    pub layout_sender: Arc<LS>,
    pub display_sender: Arc<DS>,
    pub ui_sender: Arc<US>,
    pub canvas_factory: Fact,
}

impl<
    Fact: crate::shared::rendering::ports::canvas::CanvasFactory + 'static,
    LS: LayoutEventSender + 'static,
    DS: DisplayCommandSender + 'static,
    US: UiCommandSender + 'static,
> ModuleRuntimeDependencies<Fact, LS, DS, US>
{
    #[must_use]
    pub fn new(
        hub: Arc<SignalHub>,
        surface_manager: DynSurfaceManager,
        layout_sender: Arc<LS>,
        display_sender: Arc<DS>,
        ui_sender: Arc<US>,
        canvas_factory: Fact,
    ) -> Self {
        Self {
            hub,
            surface_manager,
            layout_sender,
            display_sender,
            ui_sender,
            canvas_factory,
        }
    }
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
    ) -> Result<(), ModuleInitError> {
        self.call_function_with_args(name, &[])
    }

    /// Invoke a named function on the script with string arguments (e.g. monitor name).
    ///
    /// # Errors
    ///
    /// Returns `ModuleInitError` if the function execution fails.
    fn call_function_with_args(
        &mut self,
        name: &crate::shared::primitives::FunctionName,
        args: &[&str],
    ) -> Result<(), ModuleInitError>;
}

#[async_trait]
#[cfg_attr(test, mockall::automock)]
pub trait ModuleRegistryPort<
    Fact: crate::shared::rendering::ports::canvas::CanvasFactory + 'static,
    LS: LayoutEventSender + 'static,
    DS: DisplayCommandSender + 'static,
    US: UiCommandSender + 'static,
>: Send + Sync
{
    /// Load module configurations into registry.
    ///
    /// # Errors
    ///
    /// Returns `RegistryLoadError` if module loading fails.
    fn load(&mut self, config: &Config) -> Result<(), RegistryLoadError>;

    fn spawn_all(
        &mut self,
        deps: &ModuleRuntimeDependencies<Fact, LS, DS, US>,
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
        deps: &ModuleRuntimeDependencies<Fact, LS, DS, US>,
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
