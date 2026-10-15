use super::super::ids::{ModuleInstanceId, ModuleName};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Strongly-typed key identifying a module invocation (name + optional instance ID)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ModuleKey {
    name: ModuleName,
    instance_id: Option<ModuleInstanceId>,
}

impl ModuleKey {
    #[must_use]
    pub const fn new(name: ModuleName, instance_id: Option<ModuleInstanceId>) -> Self {
        Self { name, instance_id }
    }

    #[must_use]
    pub fn from_name(name: impl Into<ModuleName>) -> Self {
        Self {
            name: name.into(),
            instance_id: None,
        }
    }

    #[must_use]
    pub const fn name(&self) -> &ModuleName {
        &self.name
    }

    #[must_use]
    pub const fn instance_id(&self) -> Option<&ModuleInstanceId> {
        self.instance_id.as_ref()
    }
}

impl fmt::Display for ModuleKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(id) = &self.instance_id {
            write!(f, "{}:{id}", self.name)
        } else {
            write!(f, "{}", self.name)
        }
    }
}
