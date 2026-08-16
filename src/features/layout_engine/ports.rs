use crate::features::layout_engine::domain::{LayoutNode, RenderNode, TextMeasurer, LayoutError};
use crate::shared::primitives::geometry::Position;

pub trait LayoutEnginePort {
    fn calculate_layout(
        &mut self,
        node: LayoutNode,
        measurer: &mut dyn TextMeasurer,
        start_pos: Position,
    ) -> Result<RenderNode, LayoutError>;
}
