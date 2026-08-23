use crate::features::layout_engine::domain::{LayoutError, RenderNode, StyledNode, TextMeasurer};
use crate::shared::primitives::geometry::Position;

pub trait LayoutEnginePort {
    fn calculate_layout(
        &mut self,
        node: StyledNode,
        measurer: &mut dyn TextMeasurer,
        start_pos: Position,
    ) -> Result<RenderNode, LayoutError>;
}
