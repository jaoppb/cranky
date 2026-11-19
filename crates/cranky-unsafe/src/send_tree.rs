use std::ops::{Deref, DerefMut};
use taffy::TaffyTree;

/// A [`TaffyTree`] that may be sent to, and shared with, another thread.
///
/// `TaffyTree` is neither `Send` nor `Sync` because a style can hold a raw
/// pointer for a `calc()` value.
#[derive(Debug, Default)]
pub struct SendTaffyTree {
    tree: TaffyTree,
}

// SAFETY: the only non-`Send` data a `TaffyTree` owns is the raw pointer of a
// `calc()` dimension. A `TaffyTree<()>` never dereferences it: its
// `resolve_calc_value` ignores the pointer and returns `0.0`. The pointer is
// therefore an inert number, whatever style a caller stores through
// `DerefMut`, and the tree holds nothing tied to the thread that built it.
unsafe impl Send for SendTaffyTree {}
// SAFETY: as above; shared references to the tree only read plain owned data.
unsafe impl Sync for SendTaffyTree {}

impl SendTaffyTree {
    /// An empty tree.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl Deref for SendTaffyTree {
    type Target = TaffyTree;

    fn deref(&self) -> &Self::Target {
        &self.tree
    }
}

impl DerefMut for SendTaffyTree {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.tree
    }
}

#[cfg(test)]
mod tests {
    use super::SendTaffyTree;

    #[test]
    fn a_tree_built_on_one_thread_is_usable_on_another() {
        let mut tree = SendTaffyTree::new();
        tree.new_leaf(taffy::Style::default()).unwrap();

        let total = std::thread::spawn(move || tree.total_node_count())
            .join()
            .unwrap();

        assert_eq!(total, 1);
    }
}
