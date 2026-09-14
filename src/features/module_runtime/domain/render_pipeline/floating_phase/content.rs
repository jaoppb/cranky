use super::super::measurer::ModuleSizeMeasurer;
use crate::features::layout_engine::domain::{RenderNode, StyledNode};
use crate::features::layout_engine::ports::LayoutEnginePort;
use crate::shared::config::domain::{FontFamily, FontSize};
use crate::shared::primitives::geometry::{Position, Scale};
use crate::shared::primitives::ChildSizesMap;
use crate::shared::rendering::ports::canvas::CanvasFactory;

/// Fallback font used when a floating node's own CSS specifies none —
/// matches `paint_pipeline`'s fallback so a popup's fonts never disagree
/// with what the bar tree would use for the same unstyled node.
const fn floating_font_fallback() -> (FontFamily, FontSize) {
    (FontFamily::new(String::new()), FontSize::new(14.0))
}

/// Lays out one piece of floating content (a popup's, panel's or tooltip's)
/// with the pipeline's own measurer, scale and font fallback — shared by all
/// three so hit-testing and painting always agree, whichever kind it is.
pub(super) fn layout_floating_content<F: CanvasFactory>(
    layout_engine: &mut dyn LayoutEnginePort,
    canvas_factory: &mut F,
    scale: Scale,
    current_child_sizes: Option<&ChildSizesMap>,
    content: &StyledNode,
) -> Option<RenderNode> {
    let (font_family, font_size) = floating_font_fallback();
    let measurer_inner = canvas_factory.create_text_measurer(scale, font_family, font_size);
    let mut measurer = ModuleSizeMeasurer::new(measurer_inner, current_child_sizes);
    match layout_engine.calculate_layout(content.clone(), &mut measurer, Position::new(0, 0)) {
        Ok(node) => Some(node),
        Err(e) => {
            tracing::error!(err = ?e, "Floating content layout calculation failed");
            None
        }
    }
}
