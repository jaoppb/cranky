use super::content::layout_floating_content;
use crate::features::layout_engine::domain::RenderNode;
use crate::features::layout_engine::ports::LayoutEnginePort;
use crate::features::vdom::domain::{InteractionContext, NodePath, SurfaceSpace};
use crate::shared::primitives::geometry::Scale;
use crate::shared::primitives::ChildSizesMap;
use crate::shared::rendering::ports::canvas::CanvasFactory;

/// Picks the render tree that owns `hovered_path`, so the tooltip lookup
/// walks the same surface the hover actually happened on.
const fn hovered_tree<'a>(
    bar: &'a RenderNode,
    popup: Option<&'a RenderNode>,
    panel: Option<&'a RenderNode>,
    hovered_path: &NodePath,
) -> Option<&'a RenderNode> {
    match hovered_path.surface() {
        SurfaceSpace::Bar => Some(bar),
        SurfaceSpace::Popup => popup,
        SurfaceSpace::Panel => panel,
        // A tooltip cannot itself own the hover that opens a tooltip —
        // tooltip content isn't hit-tested (it receives no pointer events).
        SurfaceSpace::Tooltip => None,
    }
}

/// Resolves whichever tooltip is active for the current hover, and lays it
/// out. Mirrors `PointerHandler`'s old `hit.iter().rev().find_map(|n|
/// n.tooltip())` — the deepest node on the hovered path that carries a
/// tooltip wins — but works from `hovered_path` against the already-laid-out
/// trees instead of a hit-test result, since the pipeline never hit-tests.
#[allow(clippy::too_many_arguments)]
pub(super) fn layout_tooltip<F: CanvasFactory>(
    bar: &RenderNode,
    popup: Option<&RenderNode>,
    panel: Option<&RenderNode>,
    interaction: Option<&InteractionContext>,
    layout_engine: &mut dyn LayoutEnginePort,
    canvas_factory: &mut F,
    scale: Scale,
    current_child_sizes: Option<&ChildSizesMap>,
) -> Option<RenderNode> {
    let hovered_path = interaction?.hovered_path()?;
    let tree = hovered_tree(bar, popup, panel, hovered_path)?;
    let tooltip = tree.find_tooltip_along(hovered_path)?;
    layout_floating_content(layout_engine, canvas_factory, scale, current_child_sizes, tooltip)
}
