use super::errors::ModuleInitError;
use crate::shared::config::domain::{Config, ModuleConfig};
use crate::shared::events::signals::{SignalHub, SignalKind};
use crate::shared::primitives::MonitorId;
use async_trait::async_trait;

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
