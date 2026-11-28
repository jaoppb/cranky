use crate::features::layout_engine::domain::LayoutError;
use taffy::TaffyTree;
use taffy::style::Style;
use taffy::tree::NodeId;

pub(super) struct TaffyTreeBuilder<'a> {
    taffy: &'a mut TaffyTree,
}

impl<'a> TaffyTreeBuilder<'a> {
    pub(crate) const fn new(taffy: &'a mut TaffyTree) -> Self {
        Self { taffy }
    }

    pub(super) fn add_leaf(&mut self, style: Style) -> Result<NodeId, LayoutError> {
        self.taffy
            .new_leaf(style)
            .map_err(|error| LayoutError::EngineError(error.to_string()))
    }

    pub(super) fn add_node(
        &mut self,
        style: Style,
        children: &[NodeId],
    ) -> Result<NodeId, LayoutError> {
        self.taffy
            .new_with_children(style, children)
            .map_err(|error| LayoutError::EngineError(error.to_string()))
    }

    pub(super) fn set_style(&mut self, node_id: NodeId, style: Style) -> Result<(), LayoutError> {
        self.taffy
            .set_style(node_id, style)
            .map_err(|error| LayoutError::EngineError(error.to_string()))
    }

    pub(super) fn set_children(
        &mut self,
        node_id: NodeId,
        children: &[NodeId],
    ) -> Result<(), LayoutError> {
        self.taffy
            .set_children(node_id, children)
            .map_err(|error| LayoutError::EngineError(error.to_string()))
    }

    /// Frees `node_id` and everything under it.
    ///
    /// `node_id` **must** still be live. Both taffy calls below index their
    /// `SlotMap` directly (`children` -> `self.children[key]`, `remove` ->
    /// `self.parents[key]`) and panic on a freed key rather than returning
    /// `Err`, so the `if let Ok(..)` guard catches nothing. Taffy exposes no
    /// way to test a key, which is why callers must never retain a freed
    /// `NodeId` in their own bookkeeping.
    pub(super) fn remove_recursive(&mut self, node_id: NodeId) {
        if let Ok(children) = self.taffy.children(node_id) {
            for child in children {
                self.remove_recursive(child);
            }
        }
        let _ = self.taffy.remove(node_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remove_recursive_frees_the_node_and_its_whole_subtree() {
        let mut taffy = TaffyTree::new();
        let mut builder = TaffyTreeBuilder::new(&mut taffy);
        let grandchild = builder.add_leaf(Style::default()).unwrap();
        let child = builder.add_node(Style::default(), &[grandchild]).unwrap();
        let sibling = builder.add_leaf(Style::default()).unwrap();
        let root = builder
            .add_node(Style::default(), &[child, sibling])
            .unwrap();
        let kept = builder.add_node(Style::default(), &[]).unwrap();

        builder.remove_recursive(root);

        assert_eq!(taffy.total_node_count(), 1);
        assert_eq!(taffy.children(kept).unwrap(), Vec::<NodeId>::new());
    }
}
