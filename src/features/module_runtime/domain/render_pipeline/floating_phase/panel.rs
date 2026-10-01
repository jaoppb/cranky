use super::content::layout_floating_content_cached;
use crate::features::layout_engine::domain::popup::ActivePanel;
use crate::features::layout_engine::domain::{RenderNode, StyledNode};
use crate::features::layout_engine::ports::LayoutEnginePort;
use crate::shared::primitives::geometry::Scale;
use crate::shared::primitives::{ChildSizesMap, MonitorId};
use crate::shared::rendering::ports::canvas::CanvasFactory;
use std::collections::HashMap;

/// Lays out an active panel's content the same way `layout_popup` does.
#[allow(clippy::too_many_arguments)]
pub(super) fn layout_panel<F: CanvasFactory, E: LayoutEnginePort>(
    active: &ActivePanel<'_>,
    cache: &mut HashMap<MonitorId, (StyledNode, RenderNode)>,
    monitor_id: &MonitorId,
    layout_engine: &mut E,
    canvas_factory: &mut F,
    scale: Scale,
    current_child_sizes: Option<&ChildSizesMap>,
) -> Option<RenderNode> {
    layout_floating_content_cached(
        cache,
        monitor_id,
        layout_engine,
        canvas_factory,
        scale,
        current_child_sizes,
        active.layout(),
    )
}
