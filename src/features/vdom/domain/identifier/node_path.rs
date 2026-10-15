use serde::{Deserialize, Serialize};

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

    /// Appends `suffix`'s indices onto this path, e.g. `[0,1].extend([2])`
    /// = `[0,1,2]`. Used to turn a relative path (from some anchor) back
    /// into an absolute one once the anchor's own absolute path is known.
    /// The surface of `self` wins — `suffix` is always relative and shares
    /// the same surface by construction.
    #[must_use]
    pub fn extend(&self, suffix: &Self) -> Self {
        let mut indices = self.indices.clone();
        indices.extend_from_slice(&suffix.indices);
        Self {
            surface: self.surface,
            indices,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn test_node_path_extend() {
        let anchor = NodePath::new(SurfaceSpace::Bar, vec![0, 1]);
        let relative = NodePath::new(SurfaceSpace::Bar, vec![2]);
        assert_eq!(anchor.extend(&relative).as_slice(), &[0, 1, 2]);
        assert_eq!(
            NodePath::root_in(SurfaceSpace::Bar)
                .extend(&relative)
                .as_slice(),
            &[2]
        );
    }
}
