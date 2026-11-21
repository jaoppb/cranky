use super::super::FloatingPass;
use super::super::tooltip::{HoverTrees, layout_tooltip};
use super::{RecordingEngine, Spec, tooltip_content};
use crate::features::layout_engine::domain::{RenderNode, StyledNode};
use crate::features::module_runtime::test_support::MockCanvasFactory;
use crate::features::vdom::domain::{
    AnchorSegment, InteractionContext, NodeKey, NodePath, NodeRef, SurfaceSpace,
};
use crate::shared::primitives::geometry::Scale;
use crate::shared::primitives::{MonitorFocus, MonitorId};
use std::collections::HashMap;

/// A surface's root node, carrying `tip` as its own tooltip.
fn root(surface: SurfaceSpace, tip: Option<&str>, children: Vec<RenderNode>) -> RenderNode {
    Spec {
        tooltip: tip.map(tooltip_content),
        ..Spec::at_path(surface, &[])
    }
    .flex(children)
}

/// The bar's child at `index`, optionally keyed, carrying `tip`.
fn child(index: usize, key: Option<&'static str>, tip: Option<&str>) -> RenderNode {
    Spec {
        key,
        tooltip: tip.map(tooltip_content),
        ..Spec::at_path(SurfaceSpace::Bar, &[index])
    }
    .leaf()
}

fn hover(node_ref: NodeRef) -> InteractionContext {
    InteractionContext::new(Some(node_ref), None, None, MonitorFocus::Focused)
}

fn unkeyed(surface: SurfaceSpace, path: &[usize]) -> NodeRef {
    NodeRef::new(Vec::new(), NodePath::new(surface, path.to_vec()))
}

/// The tooltip contents handed to the layout engine for `interaction`.
fn picked(
    bar: &RenderNode,
    popup: Option<&RenderNode>,
    panel: Option<&RenderNode>,
    interaction: Option<&InteractionContext>,
) -> Vec<StyledNode> {
    let mut engine = RecordingEngine::default();
    let _ = layout_tooltip(
        HoverTrees { bar, popup, panel },
        interaction,
        &mut HashMap::new(),
        &mut engine,
        &mut FloatingPass {
            monitor_id: &MonitorId::new("DP-1"),
            canvas_factory: &mut MockCanvasFactory,
            scale: Scale::new(1.0),
            current_child_sizes: None,
        },
    );
    engine.laid_out
}

#[test]
fn no_hover_lays_out_no_tooltip() {
    let bar = root(SurfaceSpace::Bar, Some("bar"), vec![]);
    assert_eq!(picked(&bar, None, None, None), vec![]);
    let idle = InteractionContext::new(None, None, None, MonitorFocus::Focused);
    assert_eq!(picked(&bar, None, None, Some(&idle)), vec![]);
}

#[test]
fn deepest_tooltip_on_the_hovered_path_wins() {
    let bar = root(
        SurfaceSpace::Bar,
        Some("outer"),
        vec![child(0, None, Some("first")), child(1, None, None)],
    );

    let on_first = hover(unkeyed(SurfaceSpace::Bar, &[0]));
    assert_eq!(
        picked(&bar, None, None, Some(&on_first)),
        vec![tooltip_content("first")]
    );
    // The second child has none of its own, so its ancestor's shows.
    let on_second = hover(unkeyed(SurfaceSpace::Bar, &[1]));
    assert_eq!(
        picked(&bar, None, None, Some(&on_second)),
        vec![tooltip_content("outer")]
    );
}

#[test]
fn tooltip_is_looked_up_in_the_surface_the_hover_happened_on() {
    let bar = root(SurfaceSpace::Bar, Some("bar"), vec![]);
    let popup = root(SurfaceSpace::Popup, Some("popup"), vec![]);
    let panel = root(SurfaceSpace::Panel, Some("panel"), vec![]);

    for (surface, expected) in [
        (SurfaceSpace::Bar, "bar"),
        (SurfaceSpace::Popup, "popup"),
        (SurfaceSpace::Panel, "panel"),
    ] {
        let ctx = hover(unkeyed(surface, &[]));
        assert_eq!(
            picked(&bar, Some(&popup), Some(&panel), Some(&ctx)),
            vec![tooltip_content(expected)],
            "{surface:?}"
        );
    }
}

#[test]
fn hover_on_a_surface_that_is_not_open_lays_out_nothing() {
    let bar = root(SurfaceSpace::Bar, Some("bar"), vec![]);
    for surface in [
        SurfaceSpace::Popup,
        SurfaceSpace::Panel,
        SurfaceSpace::Tooltip,
    ] {
        let ctx = hover(unkeyed(surface, &[]));
        assert_eq!(picked(&bar, None, None, Some(&ctx)), vec![], "{surface:?}");
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
    let bar = root(
        SurfaceSpace::Bar,
        None,
        vec![
            child(0, Some("a"), Some("tip a")),
            child(1, Some("b"), Some("tip b")),
        ],
    );
    assert_eq!(
        picked(&bar, None, None, Some(&hover(captured.clone()))),
        vec![tooltip_content("tip a")]
    );

    // Once the keyed node is gone the hover resolves to nothing, rather
    // than to whichever node now sits at the captured position.
    let without_a = root(
        SurfaceSpace::Bar,
        None,
        vec![
            child(0, Some("c"), Some("tip c")),
            child(1, Some("b"), Some("tip b")),
        ],
    );
    assert_eq!(
        picked(&without_a, None, None, Some(&hover(captured))),
        vec![]
    );
}
