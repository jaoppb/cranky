//! `AnyModulePort`, the one interface the runtime sees for a script module, whatever its engine.
use super::errors::ModuleInitError;
use crate::shared::config::domain::{Config, ModuleConfig};
use crate::shared::events::signals::{SignalHub, SignalKind};
use crate::shared::primitives::MonitorId;
use async_trait::async_trait;

/// The interface the runtime drives every script module through; the Lua and Rhai adapters both
/// implement it.
#[async_trait]
pub trait AnyModulePort: Send + Sync {
    /// Initialize module with configuration.
    ///
    /// # Errors
    ///
    /// Returns `ModuleInitError` if module initialization fails.
    fn init(&mut self, config: &ModuleConfig, full_config: &Config) -> Result<(), ModuleInitError>;
    /// Signals whose changes wake this module; it is never refreshed for any other.
    fn subscriptions(&self) -> &[SignalKind];
    /// `DBus` signals this module wants delivered, from its `metadata()`; none by default.
    fn dbus_subscriptions(&self) -> &[crate::shared::dbus::domain::DBusSubscription] {
        &[]
    }
    /// Stylesheets this module's tree is styled with, as declared in its `metadata().styles`.
    fn styles(&self) -> &[crate::features::styling::domain::StyleSheetName];
    /// Lets the script update its state from the hub after `changed_signals` fired.
    fn refresh(&mut self, hub: &SignalHub, changed_signals: &[SignalKind]);
    /// Builds the module's `VNode` tree for one monitor, from the state the last `refresh` left.
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
