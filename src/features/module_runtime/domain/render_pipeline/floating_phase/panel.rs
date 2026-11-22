use super::content::{FloatingEngine, FloatingPass, layout_floating_content_cached};
use crate::features::layout_engine::domain::RenderNode;
use crate::features::layout_engine::domain::popup::ActivePanel;
use crate::features::layout_engine::ports::LayoutEnginePort;
use crate::shared::rendering::ports::canvas::CanvasFactory;

/// Lays out an active panel's content the same way `layout_popup` does.
pub(super) fn layout_panel<F: CanvasFactory, E: LayoutEnginePort>(
    active: &ActivePanel<'_>,
    engine: FloatingEngine<'_, E>,
    pass: &mut FloatingPass<'_, F>,
) -> Option<RenderNode> {
    layout_floating_content_cached(engine, pass, active.layout())
}
