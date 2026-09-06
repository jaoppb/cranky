use super::geometry::{Rect, Size};
use super::ids::{ModuleInstanceId, ModuleName};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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

/// Strongly-typed layout descriptor for a child module in a container
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChildModuleLayout {
    key: ModuleKey,
    bounds: Rect,
}

impl ChildModuleLayout {
    #[must_use]
    pub const fn new(key: ModuleKey, bounds: Rect) -> Self {
        Self { key, bounds }
    }

    #[must_use]
    pub const fn key(&self) -> &ModuleKey {
        &self.key
    }

    #[must_use]
    pub const fn bounds(&self) -> &Rect {
        &self.bounds
    }
}

/// Strongly-typed map of child module sizes per monitor
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ChildSizesMap(HashMap<ModuleKey, Size>);

impl ChildSizesMap {
    #[must_use]
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn insert(&mut self, key: ModuleKey, size: Size) {
        self.0.insert(key, size);
    }

    #[must_use]
    pub fn get(&self, key: &ModuleKey) -> Option<&Size> {
        self.0.get(key)
    }

    #[must_use]
    pub fn get_by_name_or_key(
        &self,
        name: &ModuleName,
        instance_id: Option<&ModuleInstanceId>,
    ) -> Option<&Size> {
        let key = ModuleKey::new(name.clone(), instance_id.cloned());
        self.0.get(&key).or_else(|| {
            if instance_id.is_some() {
                self.0.get(&ModuleKey::new(name.clone(), None))
            } else {
                None
            }
        })
    }
}
