use super::errors::VdomError;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NodeId(Uuid);

impl NodeId {
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    #[must_use]
    pub const fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    #[must_use]
    pub const fn uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Default for NodeId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Which floating surface (or the bar itself) a `NodePath` lives in.
///
/// A bar node and a popup node can share the same numeric indices — they're
/// different trees. Bundling the surface into the path keeps every path
/// globally unique, and keeps ancestor matching (`starts_with`, used for
/// `:hover`/`:active` propagation) from crossing a surface boundary: hovering
/// a node inside a popup must never mark the popup's owner node `:hover`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum SurfaceSpace {
    #[default]
    Bar,
    Popup,
    Panel,
    Tooltip,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct NodePath {
    surface: SurfaceSpace,
    indices: Vec<usize>,
}

impl NodePath {
    /// The root of the given surface's own tree. There is no argument-less
    /// `root()` — every construction site names its surface on purpose, so a
    /// path built for the wrong tree is a compile error, not a runtime bug.
    #[must_use]
    pub const fn root_in(surface: SurfaceSpace) -> Self {
        Self {
            surface,
            indices: Vec::new(),
        }
    }

    #[must_use]
    pub const fn new(surface: SurfaceSpace, indices: Vec<usize>) -> Self {
        Self { surface, indices }
    }

    #[must_use]
    pub fn child(&self, index: usize) -> Self {
        let mut indices = self.indices.clone();
        indices.push(index);
        Self {
            surface: self.surface,
            indices,
        }
    }

    #[must_use]
    pub fn as_slice(&self) -> &[usize] {
        &self.indices
    }

    #[must_use]
    pub const fn surface(&self) -> SurfaceSpace {
        self.surface
    }

    /// The root of its own surface — a popup's root is still root, not a
    /// descendant of whatever node opened it.
    #[must_use]
    pub const fn is_root(&self) -> bool {
        self.indices.is_empty()
    }

    #[must_use]
    pub fn starts_with(&self, prefix: &Self) -> bool {
        self.surface == prefix.surface && self.indices.starts_with(&prefix.indices)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InteractionContext {
    hovered_path: Option<NodePath>,
    active_path: Option<NodePath>,
    focused_path: Option<NodePath>,
    is_monitor_focused: bool,
}

impl InteractionContext {
    #[must_use]
    pub const fn new(
        hovered_path: Option<NodePath>,
        active_path: Option<NodePath>,
        focused_path: Option<NodePath>,
        is_monitor_focused: bool,
    ) -> Self {
        Self {
            hovered_path,
            active_path,
            focused_path,
            is_monitor_focused,
        }
    }

    #[must_use]
    pub const fn hovered_path(&self) -> Option<&NodePath> {
        self.hovered_path.as_ref()
    }

    #[must_use]
    pub const fn active_path(&self) -> Option<&NodePath> {
        self.active_path.as_ref()
    }

    #[must_use]
    pub const fn focused_path(&self) -> Option<&NodePath> {
        self.focused_path.as_ref()
    }

    #[must_use]
    pub const fn is_monitor_focused(&self) -> bool {
        self.is_monitor_focused
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct NodeKey(String);

impl NodeKey {
    /// Creates a new `NodeKey`.
    ///
    /// # Errors
    ///
    /// Returns `VdomError::InvalidNodeKey` if the key is empty or whitespace.
    pub fn new(key: impl Into<String>) -> Result<Self, VdomError> {
        let s = key.into();
        if s.trim().is_empty() {
            return Err(VdomError::InvalidNodeKey(
                "NodeKey cannot be empty or whitespace".to_string(),
            ));
        }
        Ok(Self(s))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for NodeKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<'de> Deserialize<'de> for NodeKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Self::new(s).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_id_creation_and_uniqueness() {
        let id1 = NodeId::new();
        let id2 = NodeId::new();
        assert_ne!(id1, id2);
        assert_eq!(id1, NodeId::from_uuid(*id1.uuid()));
        assert!(!id1.to_string().is_empty());
    }

    #[test]
    fn test_node_key_validation() {
        assert!(NodeKey::new("valid-key_123").is_ok());
        assert!(NodeKey::new("").is_err());
        assert!(NodeKey::new("   ").is_err());

        let key = NodeKey::new("tab-1").unwrap();
        assert_eq!(key.as_str(), "tab-1");
        assert_eq!(key.to_string(), "tab-1");
    }

    #[test]
    fn test_node_path_operations() {
        let root = NodePath::root_in(SurfaceSpace::Bar);
        assert!(root.is_root());
        let empty: &[usize] = &[];
        assert_eq!(root.as_slice(), empty);

        let child0 = root.child(0);
        assert!(!child0.is_root());
        assert_eq!(child0.as_slice(), &[0]);
        assert!(child0.starts_with(&root));

        let child0_1 = child0.child(1);
        assert_eq!(child0_1.as_slice(), &[0, 1]);
        assert!(child0_1.starts_with(&child0));
        assert!(child0_1.starts_with(&root));

        let child1 = root.child(1);
        assert!(!child0_1.starts_with(&child1));
    }

    #[test]
    fn test_node_path_surface_isolation() {
        let bar_root = NodePath::root_in(SurfaceSpace::Bar);
        let popup_root = NodePath::root_in(SurfaceSpace::Popup);
        assert_ne!(bar_root, popup_root);

        let bar_child = bar_root.child(0).child(1);
        let popup_child = popup_root.child(0).child(1);
        // Same indices, different surfaces: not equal, and neither is an
        // ancestor of the other.
        assert_ne!(bar_child, popup_child);
        assert!(!popup_child.starts_with(&bar_root));
        assert!(!bar_child.starts_with(&popup_root));

        assert!(popup_root.is_root());
        assert_eq!(popup_root.surface(), SurfaceSpace::Popup);
    }
}
