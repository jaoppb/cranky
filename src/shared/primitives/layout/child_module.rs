//! Layout descriptors for child modules embedded in a container, and the size reports
//! children send back to their embedder.
use super::super::geometry::{Rect, Size};
use super::super::ids::{ModuleInstanceId, ModuleName};
use super::super::options::ModuleOptions;
use super::module_key::ModuleKey;
use super::size_constraint::SizeConstraint;
use super::surface::LayoutSurface;
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
    /// Which tree this child was embedded in — the bar's own tree, or one of
    /// the three floating kinds. Decides which physical Wayland surface the
    /// child's own subsurface is parented to.
    surface: LayoutSurface,
}

impl ChildModuleLayout {
    #[must_use]
    pub(crate) const fn new(
        key: ModuleKey,
        bounds: Rect,
        constraint: SizeConstraint,
        options: ModuleOptions,
        surface: LayoutSurface,
    ) -> Self {
        Self {
            key,
            bounds,
            constraint,
            options,
            surface,
        }
    }

    #[must_use]
    pub(crate) const fn key(&self) -> &ModuleKey {
        &self.key
    }

    #[must_use]
    pub(crate) const fn bounds(&self) -> &Rect {
        &self.bounds
    }

    #[must_use]
    pub(crate) const fn constraint(&self) -> SizeConstraint {
        self.constraint
    }

    #[must_use]
    pub(crate) const fn options(&self) -> &ModuleOptions {
        &self.options
    }

    #[must_use]
    pub(crate) const fn surface(&self) -> LayoutSurface {
        self.surface
    }
}

/// What a child module has reported to its embedder.
///
/// `size` is intrinsic (laid out with no pin, so it never depends on the
/// width the parent chose); `settled_width` is the width it was last laid
/// out at once a parent pinned one. The parent measures with `size` and gates a floating surface on
/// `settled_width`, so the child's pixels are never shown at a stale width.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ChildReport {
    size: Size,
    settled_width: Option<u32>,
}

impl ChildReport {
    #[must_use]
    pub(crate) const fn new(size: Size, settled_width: Option<u32>) -> Self {
        Self {
            size,
            settled_width,
        }
    }

    #[must_use]
    pub(crate) const fn size(&self) -> &Size {
        &self.size
    }

    #[must_use]
    pub(crate) const fn settled_width(&self) -> Option<u32> {
        self.settled_width
    }
}

/// Strongly-typed map of child module reports per monitor
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ChildSizesMap(HashMap<ModuleKey, ChildReport>);

impl ChildSizesMap {
    #[cfg(test)]
    #[must_use]
    pub(crate) fn new() -> Self {
        Self(HashMap::new())
    }

    #[cfg(test)]
    pub(crate) fn insert(&mut self, key: ModuleKey, size: Size) {
        self.insert_report(key, ChildReport::new(size, None));
    }

    pub(crate) fn insert_report(&mut self, key: ModuleKey, report: ChildReport) {
        self.0.insert(key, report);
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) fn get(&self, key: &ModuleKey) -> Option<&Size> {
        self.0.get(key).map(ChildReport::size)
    }

    #[must_use]
    pub(crate) fn get_by_name_or_key(
        &self,
        name: &ModuleName,
        instance_id: Option<&ModuleInstanceId>,
    ) -> Option<&Size> {
        self.report_by_name_or_key(name, instance_id)
            .map(ChildReport::size)
    }

    #[must_use]
    pub(crate) fn report_by_name_or_key(
        &self,
        name: &ModuleName,
        instance_id: Option<&ModuleInstanceId>,
    ) -> Option<&ChildReport> {
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
