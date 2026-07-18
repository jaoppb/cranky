use crate::domain::layout::{LayoutNode, RenderNode, TextMeasurer, LayoutError};
use crate::domain::shared::geometry::Position;

pub trait LayoutEnginePort: Send + Sync {
    fn calculate_layout(
        &self,
        node: LayoutNode,
        measurer: &mut dyn TextMeasurer,
        start_pos: Position,
    ) -> Result<RenderNode, LayoutError>;
}
