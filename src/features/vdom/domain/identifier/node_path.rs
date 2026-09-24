use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct NodePath(Vec<usize>);

impl NodePath {
    #[must_use]
    pub const fn root() -> Self {
        Self(Vec::new())
    }

    #[must_use]
    pub const fn new(path: Vec<usize>) -> Self {
        Self(path)
    }

    #[must_use]
    pub fn child(&self, index: usize) -> Self {
        let mut new_path = self.0.clone();
        new_path.push(index);
        Self(new_path)
    }

    #[must_use]
    pub fn as_slice(&self) -> &[usize] {
        &self.0
    }

    #[must_use]
    pub const fn is_root(&self) -> bool {
        self.0.is_empty()
    }

    #[must_use]
    pub fn starts_with(&self, prefix: &Self) -> bool {
        self.0.starts_with(&prefix.0)
    }

    /// Appends `suffix`'s indices onto this path, e.g. `[0,1].extend([2])`
    /// = `[0,1,2]`. Used to turn a relative path (from some anchor) back
    /// into an absolute one once the anchor's own absolute path is known.
    #[must_use]
    pub fn extend(&self, suffix: &Self) -> Self {
        let mut combined = self.0.clone();
        combined.extend_from_slice(&suffix.0);
        Self(combined)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_path_operations() {
        let root = NodePath::root();
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
    fn test_node_path_extend() {
        let anchor = NodePath::new(vec![0, 1]);
        let relative = NodePath::new(vec![2]);
        assert_eq!(anchor.extend(&relative).as_slice(), &[0, 1, 2]);
        assert_eq!(NodePath::root().extend(&relative).as_slice(), &[2]);
    }
}
