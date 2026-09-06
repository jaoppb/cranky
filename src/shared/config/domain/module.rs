use std::collections::HashMap;

use crate::shared::primitives::{ModuleName, ModuleOptions};

use super::types::EngineSelection;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ModulesConfig {
    modules: HashMap<ModuleName, ModuleConfig>,
}

impl ModulesConfig {
    #[must_use]
    pub const fn new(modules: HashMap<ModuleName, ModuleConfig>) -> Self {
        Self { modules }
    }

    #[must_use]
    pub fn get(&self, name: &ModuleName) -> Option<&ModuleConfig> {
        self.modules.get(name)
    }

    #[must_use]
    pub const fn modules(&self) -> &HashMap<ModuleName, ModuleConfig> {
        &self.modules
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModuleConfig {
    name: ModuleName,
    enable: bool,
    engine: EngineSelection,
    options: ModuleOptions,
}

impl ModuleConfig {
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
    pub const fn name(&self) -> &ModuleName {
        &self.name
    }

    #[must_use]
    pub const fn is_enabled(&self) -> bool {
        self.enable
    }

    #[must_use]
    pub const fn engine(&self) -> &EngineSelection {
        &self.engine
    }

    #[must_use]
    pub const fn options(&self) -> &ModuleOptions {
        &self.options
    }
}
