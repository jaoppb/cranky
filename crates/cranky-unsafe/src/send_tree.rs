use std::ops::{Deref, DerefMut};
use taffy::TaffyTree;

/// A [`TaffyTree`] that may be sent to, and shared with, another thread.
///
/// `TaffyTree` is neither `Send` nor `Sync` because a style can hold a raw
/// pointer for a `calc()` value. Cranky never builds such a style: it resolves
/// `calc()` itself and gives taffy plain lengths and percentages.
#[derive(Debug, Default)]
pub struct SendTaffyTree {
    tree: TaffyTree,
}

// SAFETY: the only non-`Send` data a `TaffyTree` can own is the raw pointer of a
// `calc()` dimension, and no caller in this workspace creates one (see the type's
// documentation), so the tree owns nothing tied to the thread that built it.
unsafe impl Send for SendTaffyTree {}
// SAFETY: as above; with no raw pointer inside, shared references to the tree
// only read plain owned data.
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
        let node = tree.new_leaf(taffy::Style::default()).unwrap();

        let total = std::thread::spawn(move || tree.total_node_count())
            .join()
            .unwrap();

        assert_eq!(total, 1);
        let _ = node;
    }
}
