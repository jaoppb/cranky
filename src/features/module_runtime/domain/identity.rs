use crate::shared::primitives::{ModuleId, ModuleInstanceId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleIdentity {
    id: ModuleId,
    parent_id: Option<ModuleId>,
    instance_id: Option<ModuleInstanceId>,
}

impl ModuleIdentity {
    #[must_use]
    pub const fn new(id: ModuleId) -> Self {
        Self {
            id,
            parent_id: None,
            instance_id: None,
        }
    }

    #[must_use]
    pub const fn with_parent(mut self, parent_id: Option<ModuleId>) -> Self {
        self.parent_id = parent_id;
        self
    }

    #[must_use]
    pub fn with_instance_id(mut self, instance_id: Option<ModuleInstanceId>) -> Self {
        self.instance_id = instance_id;
        self
    }

    #[must_use]
    pub const fn id(&self) -> ModuleId {
        self.id
    }

    #[must_use]
    pub const fn parent_id(&self) -> Option<ModuleId> {
        self.parent_id
    }

    #[must_use]
    pub const fn instance_id(&self) -> Option<&ModuleInstanceId> {
        self.instance_id.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_identity_construction_and_getters() {
        let id = ModuleId::new(10);
        let identity = ModuleIdentity::new(id);
        assert_eq!(identity.id(), id);
        assert_eq!(identity.parent_id(), None);
        assert_eq!(identity.instance_id(), None);

        let parent_id = ModuleId::new(1);
        let instance_id = ModuleInstanceId::new("clock_main");
        let updated = identity
            .with_parent(Some(parent_id))
            .with_instance_id(Some(instance_id.clone()));

        assert_eq!(updated.id(), id);
        assert_eq!(updated.parent_id(), Some(parent_id));
        assert_eq!(updated.instance_id(), Some(&instance_id));
    }
}
