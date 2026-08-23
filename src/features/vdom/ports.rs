use crate::features::vdom::domain::{DiffResult, VNode};

#[cfg_attr(test, mockall::automock)]
pub trait VdomDiffPort: Send + Sync {
    /// Diffs an optional old VDOM tree against a new VDOM tree
    fn diff<'a>(&self, old_tree: Option<&'a VNode>, new_tree: &'a VNode) -> DiffResult;
}
