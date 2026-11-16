use super::content::{FloatingCache, layout_floating_content_cached};
use crate::features::layout_engine::domain::RenderNode;
use crate::features::layout_engine::ports::LayoutEnginePort;
use crate::features::vdom::domain::{
    InteractionContext, NodePath, NodeRef, SurfaceSpace, resolve_segment,
};
use crate::shared::primitives::geometry::Scale;
use crate::shared::primitives::{ChildSizesMap, MonitorId};
use crate::shared::rendering::ports::canvas::CanvasFactory;

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
    cache: &mut FloatingCache,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::layout_engine::domain::{LayoutError, StyledNode, TextMeasurer};
    use crate::features::module_runtime::test_support::MockCanvasFactory;
    use crate::features::styling::domain::ComputedStyle;
    use crate::features::vdom::domain::{AnchorSegment, NodeKey, TextContent};
    use crate::shared::primitives::SizeConstraint;
    use crate::shared::primitives::geometry::{Position, Rect, Size};
    use std::collections::HashMap;

    /// Records which content it was asked to lay out, so a test can tell
    /// which tooltip was picked.
    #[derive(Default)]
    struct RecordingEngine {
        laid_out: Vec<StyledNode>,
    }

    impl LayoutEnginePort for RecordingEngine {
        fn calculate_layout_with_constraints(
            &mut self,
            node: StyledNode,
            _measurer: &mut dyn TextMeasurer,
            _start_pos: Position,
            _constraint: SizeConstraint,
        ) -> Result<RenderNode, LayoutError> {
            self.laid_out.push(node);
            Ok(leaf(SurfaceSpace::Tooltip, None, None))
        }
    }

    fn tooltip(text: &str) -> StyledNode {
        StyledNode::Text {
            path: NodePath::root_in(SurfaceSpace::Tooltip),
            node_key: None,
            text: TextContent::new(text.to_string()),
            style: ComputedStyle::default(),
            on_click: None,
            on_hover: None,
            tooltip: None,
            popup: None,
            panel: None,
        }
    }

    fn leaf(surface: SurfaceSpace, key: Option<&str>, tip: Option<&str>) -> RenderNode {
        RenderNode::Rect {
            path: NodePath::root_in(surface),
            node_key: key.map(|k| NodeKey::new(k).unwrap()),
            rect: Rect::new(Position::new(0, 0), Size::new(1, 1)),
            style: ComputedStyle::default(),
            on_click: None,
            on_hover: None,
            tooltip: tip.map(|t| Box::new(tooltip(t))),
            popup: None,
            panel: None,
        }
    }

    fn flex(surface: SurfaceSpace, children: Vec<RenderNode>, tip: Option<&str>) -> RenderNode {
        RenderNode::Flex {
            path: NodePath::root_in(surface),
            node_key: None,
            rect: Rect::new(Position::new(0, 0), Size::new(1, 1)),
            children,
            style: ComputedStyle::default(),
            on_click: None,
            on_hover: None,
            tooltip: tip.map(|t| Box::new(tooltip(t))),
            popup: None,
            panel: None,
        }
    }

    fn hover(node_ref: NodeRef) -> InteractionContext {
        InteractionContext::new(Some(node_ref), None, None, true)
    }

    fn unkeyed(surface: SurfaceSpace, path: &[usize]) -> NodeRef {
        NodeRef::new(Vec::new(), NodePath::new(surface, path.to_vec()))
    }

    /// The tooltips laid out for `interaction`, and whether a layout came back.
    fn picked(
        bar: &RenderNode,
        popup: Option<&RenderNode>,
        panel: Option<&RenderNode>,
        interaction: Option<&InteractionContext>,
    ) -> (Vec<StyledNode>, bool) {
        let mut engine = RecordingEngine::default();
        let node = layout_tooltip(
            bar,
            popup,
            panel,
            interaction,
            &mut HashMap::new(),
            &MonitorId::new("DP-1"),
            &mut engine,
            &mut MockCanvasFactory,
            Scale::new(1.0),
            None,
        );
        (engine.laid_out, node.is_some())
    }

    #[test]
    fn no_hover_lays_out_no_tooltip() {
        let bar = flex(SurfaceSpace::Bar, vec![], Some("bar"));
        assert_eq!(picked(&bar, None, None, None), (vec![], false));
        let idle = InteractionContext::new(None, None, None, true);
        assert_eq!(picked(&bar, None, None, Some(&idle)), (vec![], false));
    }

    #[test]
    fn deepest_tooltip_on_the_hovered_path_wins() {
        let bar = flex(
            SurfaceSpace::Bar,
            vec![
                leaf(SurfaceSpace::Bar, None, Some("first")),
                leaf(SurfaceSpace::Bar, None, None),
            ],
            Some("outer"),
        );

        let on_first = hover(unkeyed(SurfaceSpace::Bar, &[0]));
        assert_eq!(
            picked(&bar, None, None, Some(&on_first)),
            (vec![tooltip("first")], true)
        );
        // The second child has none of its own, so its ancestor's shows.
        let on_second = hover(unkeyed(SurfaceSpace::Bar, &[1]));
        assert_eq!(
            picked(&bar, None, None, Some(&on_second)),
            (vec![tooltip("outer")], true)
        );
    }

    #[test]
    fn tooltip_is_looked_up_in_the_surface_the_hover_happened_on() {
        let bar = flex(SurfaceSpace::Bar, vec![], Some("bar"));
        let popup = flex(SurfaceSpace::Popup, vec![], Some("popup"));
        let panel = flex(SurfaceSpace::Panel, vec![], Some("panel"));

        for (surface, expected) in [
            (SurfaceSpace::Bar, "bar"),
            (SurfaceSpace::Popup, "popup"),
            (SurfaceSpace::Panel, "panel"),
        ] {
            let ctx = hover(unkeyed(surface, &[]));
            assert_eq!(
                picked(&bar, Some(&popup), Some(&panel), Some(&ctx)),
                (vec![tooltip(expected)], true),
                "{surface:?}"
            );
        }
    }

    #[test]
    fn hover_on_a_surface_that_is_not_open_lays_out_nothing() {
        let bar = flex(SurfaceSpace::Bar, vec![], Some("bar"));
        for surface in [
            SurfaceSpace::Popup,
            SurfaceSpace::Panel,
            SurfaceSpace::Tooltip,
        ] {
            let ctx = hover(unkeyed(surface, &[]));
            assert_eq!(
                picked(&bar, None, None, Some(&ctx)),
                (vec![], false),
                "{surface:?}"
            );
        }
    }

    #[test]
    fn keyed_hover_follows_its_node_after_a_reorder() {
        // Captured when "a" was the second child; it has since moved first.
        let captured = NodeRef::new(
            vec![AnchorSegment::new(
                NodePath::new(SurfaceSpace::Bar, vec![1]),
                NodeKey::new("a").unwrap(),
            )],
            NodePath::root_in(SurfaceSpace::Bar),
        );
        let bar = flex(
            SurfaceSpace::Bar,
            vec![
                leaf(SurfaceSpace::Bar, Some("a"), Some("tip a")),
                leaf(SurfaceSpace::Bar, Some("b"), Some("tip b")),
            ],
            None,
        );
        assert_eq!(
            picked(&bar, None, None, Some(&hover(captured.clone()))),
            (vec![tooltip("tip a")], true)
        );

        // Once the keyed node is gone the hover resolves to nothing, rather
        // than to whichever node now sits at the captured position.
        let without_a = flex(
            SurfaceSpace::Bar,
            vec![
                leaf(SurfaceSpace::Bar, Some("c"), Some("tip c")),
                leaf(SurfaceSpace::Bar, Some("b"), Some("tip b")),
            ],
            None,
        );
        assert_eq!(
            picked(&without_a, None, None, Some(&hover(captured))),
            (vec![], false)
        );
    }
}
