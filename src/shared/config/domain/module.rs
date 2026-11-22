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

/// Whether a `[modules.<name>]` entry is loaded at startup; the TOML spells it `enable = true/false`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Enablement {
    /// The module is loaded (`enable = true`, the default).
    Enabled,
    /// The module stays unloaded (`enable = false`).
    Disabled,
}

impl From<bool> for Enablement {
    fn from(enable: bool) -> Self {
        if enable {
            Self::Enabled
        } else {
            Self::Disabled
        }
    }
}

/// One `[modules.<name>]` entry: whether the module is enabled, which engine runs it and the
/// options its script sees.
#[derive(Debug, Clone, PartialEq)]
pub struct ModuleConfig {
    name: ModuleName,
    enablement: Enablement,
    engine: EngineSelection,
    options: ModuleOptions,
}

impl ModuleConfig {
    /// Builds an entry as `[modules.<name>]` reads with only options in it: enabled, and run by
    /// whichever engine loads it first.
    #[must_use]
    pub const fn new(name: ModuleName, options: ModuleOptions) -> Self {
        Self {
            name,
            enablement: Enablement::Enabled,
            engine: EngineSelection::Auto,
            options,
        }
    }

    /// This entry with its `enable` key applied.
    #[must_use]
    pub(crate) const fn with_enablement(mut self, enablement: Enablement) -> Self {
        self.enablement = enablement;
        self
    }

    /// This entry with its `engine` key applied.
    #[must_use]
    pub(crate) fn with_engine(mut self, engine: EngineSelection) -> Self {
        self.engine = engine;
        self
    }

    #[must_use]
    pub(crate) const fn name(&self) -> &ModuleName {
        &self.name
    }

    #[must_use]
    pub(crate) const fn is_enabled(&self) -> bool {
        matches!(self.enablement, Enablement::Enabled)
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
