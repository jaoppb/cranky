use super::content::layout_floating_content_cached;
use crate::features::layout_engine::domain::{RenderNode, StyledNode};
use crate::features::layout_engine::ports::LayoutEnginePort;
use crate::features::vdom::domain::{
    AnchorSegment, InteractionContext, NodePath, NodeRef, SurfaceSpace,
};
use crate::shared::primitives::{ChildSizesMap, MonitorId};
use crate::shared::primitives::geometry::Scale;
use crate::shared::rendering::ports::canvas::CanvasFactory;
use std::collections::HashMap;

/// Picks the render tree that owns the hover, so the tooltip lookup walks
/// the same surface the hover actually happened on.
const fn hovered_tree<'a>(
    bar: &'a RenderNode,
    popup: Option<&'a RenderNode>,
    panel: Option<&'a RenderNode>,
    surface: SurfaceSpace,
) -> Option<&'a RenderNode> {
    match surface {
        SurfaceSpace::Bar => Some(bar),
        SurfaceSpace::Popup => popup,
        SurfaceSpace::Panel => panel,
        // A tooltip cannot itself own the hover that opens a tooltip —
        // tooltip content isn't hit-tested (it receives no pointer events).
        SurfaceSpace::Tooltip => None,
    }
}

/// Turns a captured `NodeRef` into a concrete `NodePath` in `root`, or
/// `None` if one of its keyed levels can no longer be found there. This is
/// `styling_bridge::anchor`'s resolver, restated over an already-laid-out
/// `RenderNode` tree instead of a `VNode` one, since that's what the
/// pipeline has in hand by the time it lays out the tooltip.
fn resolve_node_ref(root: &RenderNode, node_ref: &NodeRef) -> Option<NodePath> {
    let mut node = root;
    let mut path = NodePath::root_in(node_ref.relative_path().surface());
    for segment in node_ref.anchor() {
        (node, path) = resolve_segment(node, path, segment)?;
    }
    Some(path.extend(node_ref.relative_path()))
}

fn resolve_segment<'a>(
    start: &'a RenderNode,
    start_path: NodePath,
    segment: &AnchorSegment,
) -> Option<(&'a RenderNode, NodePath)> {
    let Some((_, through)) = segment.path().as_slice().split_last() else {
        return (start.node_key() == Some(segment.key())).then_some((start, start_path));
    };

    let mut parent = start;
    let mut parent_path = start_path;
    for &idx in through {
        parent = parent.children().get(idx)?;
        parent_path = parent_path.child(idx);
    }

    parent
        .children()
        .iter()
        .enumerate()
        .find(|(_, child)| child.node_key() == Some(segment.key()))
        .map(|(idx, child)| (child, parent_path.child(idx)))
}

/// Resolves whichever tooltip is active for the current hover, and lays it
/// out. Mirrors `PointerHandler`'s old `hit.iter().rev().find_map(|n|
/// n.tooltip())` — the deepest node on the hovered path that carries a
/// tooltip wins — but works from the hovered `NodeRef` against the
/// already-laid-out trees instead of a hit-test result, since the pipeline
/// never hit-tests.
#[allow(clippy::too_many_arguments)]
pub(super) fn layout_tooltip<F: CanvasFactory, E: LayoutEnginePort>(
    bar: &RenderNode,
    popup: Option<&RenderNode>,
    panel: Option<&RenderNode>,
    interaction: Option<&InteractionContext>,
    cache: &mut HashMap<MonitorId, (StyledNode, RenderNode)>,
    monitor_id: &MonitorId,
    layout_engine: &mut E,
    canvas_factory: &mut F,
    scale: Scale,
    current_child_sizes: Option<&ChildSizesMap>,
) -> Option<RenderNode> {
    let hovered_ref = interaction?.hovered()?;
    let tree = hovered_tree(bar, popup, panel, hovered_ref.relative_path().surface())?;
    let hovered_path = resolve_node_ref(tree, hovered_ref)?;
    let tooltip = tree.find_tooltip_along(&hovered_path)?;
    layout_floating_content_cached(
        cache,
        monitor_id,
        layout_engine,
        canvas_factory,
        scale,
        current_child_sizes,
        tooltip,
    )
}
