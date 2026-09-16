use super::super::geometry::{Rect, Size};
use super::super::ids::{ModuleInstanceId, ModuleName};
use super::super::options::ModuleOptions;
use super::module_key::ModuleKey;
use super::size_constraint::SizeConstraint;
use std::collections::HashMap;

/// Strongly-typed layout descriptor for a child module in a container
#[derive(Debug, Clone, PartialEq)]
pub struct ChildModuleLayout {
    key: ModuleKey,
    bounds: Rect,
    constraint: SizeConstraint,
    /// Whatever options the embedding `ui.module(name, { ... })` call passed
    /// — carried through so a lazy spawn (no config stanza) can `init()` the
    /// actor with them, the same way a config-declared module would.
    options: ModuleOptions,
}

impl ChildModuleLayout {
    #[must_use]
    pub const fn new(
        key: ModuleKey,
        bounds: Rect,
        constraint: SizeConstraint,
        options: ModuleOptions,
    ) -> Self {
        Self {
            key,
            bounds,
            constraint,
            options,
        }
    }

    #[must_use]
    pub const fn key(&self) -> &ModuleKey {
        &self.key
    }

    #[must_use]
    pub const fn bounds(&self) -> &Rect {
        &self.bounds
    }

    #[must_use]
    pub const fn constraint(&self) -> SizeConstraint {
        self.constraint
    }

    #[must_use]
    pub const fn options(&self) -> &ModuleOptions {
        &self.options
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
