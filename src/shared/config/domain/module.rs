//! Entries of the `[modules]` table: whether each module is enabled, its scripting engine and its
//! options.
use std::collections::HashMap;

use crate::shared::primitives::{ModuleName, ModuleOptions};

use super::types::EngineSelection;

/// Configuration of every module named in `[modules]`, looked up by module name.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ModulesConfig {
    modules: HashMap<ModuleName, ModuleConfig>,
}

impl ModulesConfig {
    #[must_use]
    pub(crate) const fn new(modules: HashMap<ModuleName, ModuleConfig>) -> Self {
        Self { modules }
    }

    #[must_use]
    pub(crate) fn get(&self, name: &ModuleName) -> Option<&ModuleConfig> {
        self.modules.get(name)
    }

    #[must_use]
    pub(crate) const fn modules(&self) -> &HashMap<ModuleName, ModuleConfig> {
        &self.modules
    }
}

/// One `[modules.<name>]` entry: whether the module is enabled, which engine runs it and the
/// options its script sees.
#[derive(Debug, Clone, PartialEq)]
pub struct ModuleConfig {
    name: ModuleName,
    enable: bool,
    engine: EngineSelection,
    options: ModuleOptions,
}

impl ModuleConfig {
    /// Builds an entry from already-parsed parts; the config DTOs are its usual caller.
    #[must_use]
    pub const fn new(
        name: ModuleName,
        enable: bool,
        engine: EngineSelection,
        options: ModuleOptions,
    ) -> Self {
        Self {
            name,
            enable,
            engine,
            options,
        }
    }

    #[must_use]
    pub(crate) const fn name(&self) -> &ModuleName {
        &self.name
    }

    #[must_use]
    pub(crate) const fn is_enabled(&self) -> bool {
        self.enable
    }

    #[must_use]
    pub(crate) const fn engine(&self) -> &EngineSelection {
        &self.engine
    }

    #[must_use]
    pub(crate) const fn options(&self) -> &ModuleOptions {
        &self.options
    }
}
