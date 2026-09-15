use super::super::ids::ModuleId;
use super::module_key::ModuleKey;

/// The identity of one embedding site: a `ModuleKey` (name + instance) as
/// invoked by a specific parent, or `None` for the root, which has none.
///
/// Two parents embedding a module of the same name are two different sites
/// — and, once minted, two different `ModuleId`s — even though their
/// `ModuleKey` is identical.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ModuleSite {
    parent: Option<ModuleId>,
    key: ModuleKey,
}

impl ModuleSite {
    #[must_use]
    pub const fn new(parent: Option<ModuleId>, key: ModuleKey) -> Self {
        Self { parent, key }
    }

    #[must_use]
    pub const fn parent(&self) -> Option<ModuleId> {
        self.parent
    }

    #[must_use]
    pub const fn key(&self) -> &ModuleKey {
        &self.key
    }
}
