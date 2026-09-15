use super::dependencies::ModuleRuntimeDependencies;
use super::errors::RegistryLoadError;
use super::layout::{LayoutEventSender, LayoutSender};
use crate::features::layout_engine::domain::DisplayCommandSender;
use crate::features::vdom::domain::UiCommandSender;
use crate::shared::config::domain::Config;
use crate::shared::events::signals::SignalKind;
use crate::shared::primitives::ModuleId;
use async_trait::async_trait;

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

    /// This module's full `(name, instance_id)` — unlike `module_names`,
    /// distinguishes two sites of the same name from each other.
    fn module_keys(&self) -> &std::collections::HashMap<ModuleId, crate::shared::primitives::ModuleKey>;

    /// Resolves the concrete actor embedded at `key` by `parent` — the
    /// per-site-aware replacement for guessing via `name_to_ids().first()`,
    /// which can't tell two parents' same-named children apart.
    fn resolve_site(
        &self,
        parent: Option<ModuleId>,
        key: &crate::shared::primitives::ModuleKey,
    ) -> Option<ModuleId>;

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
