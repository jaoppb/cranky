use super::content::layout_floating_content;
use crate::features::layout_engine::domain::popup::ActivePanel;
use crate::features::layout_engine::domain::RenderNode;
use crate::features::layout_engine::ports::LayoutEnginePort;
use crate::shared::primitives::geometry::Scale;
use crate::shared::primitives::ChildSizesMap;
use crate::shared::rendering::ports::canvas::CanvasFactory;

/// Lays out an active panel's content the same way `layout_popup` does.
pub fn layout_panel<F: CanvasFactory>(
    active: &ActivePanel<'_>,
    layout_engine: &mut dyn LayoutEnginePort,
    canvas_factory: &mut F,
    scale: Scale,
    current_child_sizes: Option<&ChildSizesMap>,
) -> Option<RenderNode> {
    layout_floating_content(
        layout_engine,
        canvas_factory,
        scale,
        current_child_sizes,
        active.layout(),
    )
}
