//! `VdomDiffPort`, the tree-diffing interface the render pipeline depends on.
use crate::features::vdom::domain::{DiffResult, VNode};

/// Compares a module's previous tree with its new one, so the render pipeline can skip work when
/// nothing changed.
pub trait VdomDiffPort: Send + Sync {
    /// Diffs an optional old VDOM tree against a new VDOM tree
    fn diff<'a>(&self, old_tree: Option<&'a VNode>, new_tree: &'a VNode) -> DiffResult;
}
