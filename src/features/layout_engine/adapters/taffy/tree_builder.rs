use crate::features::layout_engine::domain::LayoutError;
use taffy::style::Style;
use taffy::tree::NodeId;
use taffy::TaffyTree;

pub struct TaffyTreeBuilder<'a> {
    taffy: &'a mut TaffyTree,
}

impl<'a> TaffyTreeBuilder<'a> {
    pub const fn new(taffy: &'a mut TaffyTree) -> Self {
        Self { taffy }
    }

    pub(super) fn add_leaf(&mut self, style: Style) -> Result<NodeId, LayoutError> {
        self.taffy
            .new_leaf(style)
            .map_err(|e| LayoutError::EngineError(e.to_string()))
    }

    pub(super) fn add_node(&mut self, style: Style, children: &[NodeId]) -> Result<NodeId, LayoutError> {
        self.taffy
            .new_with_children(style, children)
            .map_err(|e| LayoutError::EngineError(e.to_string()))
    }

    pub(super) fn set_style(&mut self, node_id: NodeId, style: Style) -> Result<(), LayoutError> {
        self.taffy
            .set_style(node_id, style)
            .map_err(|e| LayoutError::EngineError(e.to_string()))
    }

    pub(super) fn set_children(&mut self, node_id: NodeId, children: &[NodeId]) -> Result<(), LayoutError> {
        self.taffy
            .set_children(node_id, children)
            .map_err(|e| LayoutError::EngineError(e.to_string()))
    }

    pub(super) fn remove_recursive(&mut self, node_id: NodeId) {
        if let Ok(children) = self.taffy.children(node_id) {
            for child in children {
                self.remove_recursive(child);
            }
        }
        let _ = self.taffy.remove(node_id);
    }
}
