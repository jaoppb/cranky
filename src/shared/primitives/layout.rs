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

/// A parent's CSS pin on an embedded module's slot, per axis.
///
/// `Some(px)` means the parent set an explicit `width`/`height` and taffy
/// already resolved it to that many pixels for this slot; `None` means the
/// child measures that axis intrinsically. Never built from anything the
/// child reported — only from a static CSS pin — so it cannot reintroduce a
/// parent/child size feedback loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SizeConstraint {
    width: Option<u32>,
    height: Option<u32>,
}

impl SizeConstraint {
    #[must_use]
    pub const fn none() -> Self {
        Self {
            width: None,
            height: None,
        }
    }

    #[must_use]
    pub const fn new(width: Option<u32>, height: Option<u32>) -> Self {
        Self { width, height }
    }

    #[must_use]
    pub const fn width(&self) -> Option<u32> {
        self.width
    }

    #[must_use]
    pub const fn height(&self) -> Option<u32> {
        self.height
    }
}

/// A child's rect within its parent's tree, plus whatever size constraint the
/// parent's CSS placed on it. What a `LayoutSender` delivers to the child.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChildBounds {
    rect: Rect,
    constraint: SizeConstraint,
}

impl ChildBounds {
    #[must_use]
    pub const fn new(rect: Rect, constraint: SizeConstraint) -> Self {
        Self { rect, constraint }
    }

    #[must_use]
    pub const fn rect(&self) -> Rect {
        self.rect
    }

    #[must_use]
    pub const fn constraint(&self) -> SizeConstraint {
        self.constraint
    }
}

/// Strongly-typed layout descriptor for a child module in a container
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChildModuleLayout {
    key: ModuleKey,
    bounds: Rect,
    constraint: SizeConstraint,
}

impl ChildModuleLayout {
    #[must_use]
    pub const fn new(key: ModuleKey, bounds: Rect, constraint: SizeConstraint) -> Self {
        Self {
            key,
            bounds,
            constraint,
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
