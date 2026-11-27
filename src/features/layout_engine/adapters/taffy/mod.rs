//! Flexbox and grid layout on taffy, with an incremental tree reused across passes.
mod calc_resolve;
mod diff;
mod grid_converter;
mod layout_pass;
mod leaf_size;
mod reconciler;
mod render_leaf_builder;
mod render_tree_builder;
mod style_builder;
mod style_converter;
mod tree_builder;

use crate::features::layout_engine::domain::{LayoutError, LayoutRoot, RenderNode, TextMeasurer};
use crate::features::layout_engine::ports::LayoutEnginePort;
use crate::shared::primitives::geometry::Size;
use cranky_unsafe::SendTaffyTree;
use reconciler::LayoutState;

/// Layout engine backed by taffy that keeps its tree between passes and patches it from the
/// diff against the previous `StyledNode`.
pub struct TaffyLayoutAdapter {
    taffy: SendTaffyTree,
    state: Option<LayoutState>,
    /// Root size of the last pass's unpinned layout; `None` if it wasn't pinned.
    intrinsic: Option<Size>,
}

impl std::fmt::Debug for TaffyLayoutAdapter {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("TaffyLayoutAdapter")
            .field("intrinsic", &self.intrinsic)
            .finish_non_exhaustive()
    }
}

impl Default for TaffyLayoutAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl TaffyLayoutAdapter {
    #[must_use]
    pub(crate) fn new() -> Self {
        Self {
            taffy: SendTaffyTree::new(),
            state: None,
            intrinsic: None,
        }
    }

    /// Drops the cached tree and layout state so the next pass rebuilds from
    /// scratch.
    ///
    /// Every fallible step of a layout pass runs *after* `apply_patch` has
    /// already mutated the tree, so an early return would leave `self.state`
    /// naming nodes that were freed. Taffy panics on a freed `NodeId` instead
    /// of returning an error, so a half-applied pass has to be thrown away
    /// whole.
    fn reset(&mut self) {
        self.taffy = SendTaffyTree::new();
        self.state = None;
    }
}

impl LayoutEnginePort for TaffyLayoutAdapter {
    fn calculate_layout(
        &mut self,
        root: LayoutRoot,
        measurer: &mut dyn TextMeasurer,
    ) -> Result<RenderNode, LayoutError> {
        let result = self.layout_pass(&root, measurer);
        if result.is_err() {
            self.reset();
        }
        result
    }

    fn intrinsic_size(&self) -> Option<Size> {
        self.intrinsic
    }
}

#[cfg(test)]
mod tests;
