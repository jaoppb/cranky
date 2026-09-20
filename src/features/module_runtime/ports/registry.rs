use super::dependencies::ModuleRuntimeDependencies;
use super::errors::RegistryLoadError;
use super::layout::{LayoutEventSender, LayoutSender};
use crate::features::layout_engine::domain::DisplayCommandSender;
use crate::features::vdom::domain::UiCommandSender;
use crate::shared::config::domain::Config;
use crate::shared::events::signals::SignalKind;
use crate::shared::primitives::ModuleId;
use async_trait::async_trait;

/// The `ModuleId` minted for a lazy spawn, plus the sender its new actor's
/// layout channel is reached through.
///
/// What `spawn_module` hands back instead of a bare `(ModuleId, Box<dyn
/// LayoutSender>)` tuple, whose two positions carry no names of their own
/// at the call site.
pub struct SpawnedModule {
    id: ModuleId,
    sender: Box<dyn LayoutSender>,
}

impl SpawnedModule {
    #[must_use]
    pub const fn new(id: ModuleId, sender: Box<dyn LayoutSender>) -> Self {
        Self { id, sender }
    }

    #[must_use]
    pub const fn id(&self) -> ModuleId {
        self.id
    }

    #[must_use]
    pub fn into_sender(self) -> Box<dyn LayoutSender> {
        self.sender
    }
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

    /// This module's full `(name, instance_id)` — unlike `module_names`,
    /// distinguishes two sites of the same name from each other.
    fn module_keys(
        &self,
    ) -> &std::collections::HashMap<ModuleId, crate::shared::primitives::ModuleKey>;

    /// Resolves the concrete actor embedded at `key` by `parent` — the
    /// per-site-aware replacement for guessing via `name_to_ids().first()`,
    /// which can't tell two parents' same-named children apart.
    fn resolve_site(
        &self,
        parent: Option<ModuleId>,
        key: &crate::shared::primitives::ModuleKey,
    ) -> Option<ModuleId>;

    /// The real parent this site was loaded or spawned under — `None` for
    /// the root and for an unknown id. Lets a caller walk the ancestor chain
    /// (cycle detection) without reaching into the registry's own maps.
    fn parent_of(&self, id: ModuleId) -> Option<ModuleId>;

    /// Mints a `ModuleId` for a `Module` node with no existing actor and
    /// spawns it immediately, instead of waiting for the next `spawn_all`.
    /// Used for lazy discovery: rendering `ui.module(name)` with no site
    /// resolves to no actor, which is the caller's cue to spawn one here.
    ///
    /// # Errors
    ///
    /// Returns `RegistryLoadError` if the named module can't be found or its
    /// `init()` fails — the caller decides what a failed lazy spawn means
    /// (an error placeholder), unlike `load()`, where the same failure is
    /// fatal at boot.
    fn spawn_module(
        &mut self,
        parent: ModuleId,
        key: &crate::shared::primitives::ModuleKey,
        options: crate::shared::primitives::ModuleOptions,
        surface: crate::shared::primitives::LayoutSurface,
        config: &Config,
        deps: &ModuleRuntimeDependencies<Fact, LS, DS, US>,
    ) -> Result<SpawnedModule, RegistryLoadError>;

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
