use crate::features::layout_engine::domain::{LayoutError, RenderNode, StyledNode, TextMeasurer};
use crate::shared::primitives::geometry::{Position, Size};

pub trait LayoutEnginePort: Send + Sync {
    /// Calculates the layout tree for the given styled node.
    ///
    /// # Errors
    ///
    /// Returns `LayoutError` if computing the layout fails.
    fn calculate_layout(
        &mut self,
        node: StyledNode,
        measurer: &mut dyn TextMeasurer,
        start_pos: Position,
    ) -> Result<RenderNode, LayoutError> {
        self.calculate_layout_with_constraints(node, measurer, start_pos, None)
    }

    /// Calculates the layout tree for the given styled node with size constraints.
    ///
    /// # Errors
    ///
    /// Returns `LayoutError` if computing the layout fails.
    fn calculate_layout_with_constraints(
        &mut self,
        node: StyledNode,
        measurer: &mut dyn TextMeasurer,
        start_pos: Position,
        available_size: Option<Size>,
    ) -> Result<RenderNode, LayoutError>;
}
