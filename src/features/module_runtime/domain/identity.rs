use crate::shared::primitives::{LayoutSurface, ModuleId, ModuleInstanceId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleIdentity {
    id: ModuleId,
    parent_id: Option<ModuleId>,
    instance_id: Option<ModuleInstanceId>,
    /// Which of the parent's trees this site was embedded in — the bar's own
    /// tree by default, or a floating kind for a module embedded via
    /// `ui.module(name)` inside `ui.popup`/`ui.panel`/a tooltip. Fixed for
    /// the actor's whole life: identity is per embedding site (decision 5),
    /// so a site never changes which surface it belongs to.
    surface: LayoutSurface,
}

impl ModuleIdentity {
    #[must_use]
    pub const fn new(id: ModuleId) -> Self {
        Self {
            id,
            parent_id: None,
            instance_id: None,
            surface: LayoutSurface::Bar,
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
    pub const fn with_surface(mut self, surface: LayoutSurface) -> Self {
        self.surface = surface;
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

    #[must_use]
    pub const fn surface(&self) -> LayoutSurface {
        self.surface
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
