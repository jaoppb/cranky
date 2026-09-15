use crate::features::layout_engine::domain::{LayoutError, RenderNode, StyledNode, TextMeasurer};
use crate::shared::primitives::geometry::Position;
use crate::shared::primitives::SizeConstraint;

pub trait LayoutEnginePort: Send + Sync {
    /// Calculates the layout tree for the given styled node, measuring
    /// intrinsically on both axes.
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
        self.calculate_layout_with_constraints(node, measurer, start_pos, SizeConstraint::none())
    }

    /// Calculates the layout tree for the given styled node. A pinned axis on
    /// `constraint` becomes a definite available space for that axis; an
    /// unpinned axis is measured intrinsically.
    ///
    /// # Errors
    ///
    /// Returns `LayoutError` if computing the layout fails.
    fn calculate_layout_with_constraints(
        &mut self,
        node: StyledNode,
        measurer: &mut dyn TextMeasurer,
        start_pos: Position,
        constraint: SizeConstraint,
    ) -> Result<RenderNode, LayoutError>;
}
