//! The port that turns a styled node tree into positioned render nodes, implemented by the
//! taffy adapter.

use crate::features::layout_engine::domain::{LayoutError, LayoutRoot, RenderNode, TextMeasurer};
use crate::shared::primitives::geometry::Size;

/// Lays out a `StyledNode` tree into a `RenderNode` tree; an engine may keep state between
/// passes to lay out incrementally.
pub trait LayoutEnginePort: Send + Sync {
    /// Lays out `root`'s tree in its box, placing the tree at the root's origin (every rect is
    /// offset by it). A pinned axis of the box becomes a definite available space for that axis;
    /// an unpinned axis is measured intrinsically.
    ///
    /// # Errors
    ///
    /// Returns `LayoutError` if computing the layout fails.
    fn calculate_layout(
        &mut self,
        root: LayoutRoot,
        measurer: &mut dyn TextMeasurer,
    ) -> Result<RenderNode, LayoutError>;

    /// The root's size from the *unpinned* pass of the last
    /// `calculate_layout`, or `None` when that call had no
    /// pin (its result already is the intrinsic size).
    ///
    /// An embedded module reports this to its embedder rather than the
    /// pinned size: the embedder sizes the host from it, so it must not
    /// depend on the width the embedder chose.
    fn intrinsic_size(&self) -> Option<Size> {
        None
    }
}
