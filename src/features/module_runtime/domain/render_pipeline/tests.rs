use super::context::LayoutContext;
use super::outcome::SizeChange;
use super::pipeline::RenderPipeline;
use crate::features::layout_engine::domain::RenderNode;
use crate::features::module_runtime::test_support::{
    MockCanvasFactory, MockLayoutEngine, MockStyleResolver, MockVdomDiff, PipelineFixture,
    TestModulePort, process_monitor,
};
use crate::features::vdom::domain::{ElementSelectors, Interactions, VNode};
use crate::shared::primitives::MonitorId;
use crate::shared::primitives::geometry::{Position, Rect, Scale, Size};
use std::collections::HashMap;

#[test]
fn test_pipeline_diff_and_layout_phases() {
    let mut pipeline = RenderPipeline::new();
    let mon = MonitorId::new("DP-1");
    let rect = VNode::new_rect(ElementSelectors::default(), Interactions::default());
    let port = TestModulePort::new(rect);
    let diff_adapter = MockVdomDiff;
    let resolver = MockStyleResolver;
    let mut factory = MockCanvasFactory;
    let mut engine = MockLayoutEngine;
    let mut popup_engine = MockLayoutEngine;
    let mut panel_engine = MockLayoutEngine;
    let mut tooltip_engine = MockLayoutEngine;
    let child_errors = HashMap::new();

    let mut ctx = LayoutContext {
        monitor_id: &mon,
        scale: Scale::new(1.0),
        style_resolver: &resolver,
        child_errors: &child_errors,
        current_bounds: None,
        current_constraint: crate::shared::primitives::SizeConstraint::none(),
        current_child_sizes: None,
        interaction_context: None,
        canvas_factory: &mut factory,
        layout_engine: &mut engine,
        popup_layout_engine: &mut popup_engine,
        panel_layout_engine: &mut panel_engine,
        tooltip_layout_engine: &mut tooltip_engine,
    };
    // 1. Diff Phase
    let diff = pipeline.diff(&port, ctx.diff_input(&diff_adapter));
    assert!(diff.is_some());
    let diff = diff.unwrap();
    assert!(diff.vdom_dirty());

    // 2. Layout Phase
    let layout_res = pipeline.layout(diff, &mut ctx);
    assert!(layout_res.is_some());
    let layout = layout_res.unwrap();
    let node = layout.render_tree();
    assert!(layout.floating().popup().is_none());
    assert!(layout.floating().panel().is_none());
    assert_eq!(
        layout.report().size_change(),
        Some(&SizeChange::new(Size::new(0, 0), Size::new(10, 10)))
    );
    assert_eq!(layout.report().child_layouts(), []);
    assert_eq!(*node.rect().size(), Size::new(10, 10));

    // 3. Paint Phase without bounds returns None
    let paint_res = pipeline.paint(node, &mut ctx);
    assert!(paint_res.is_none());

    // 4. Paint Phase with valid bounds returns buffer
    ctx.current_bounds = Some(Rect::new(Position::new(0, 0), Size::new(10, 10)));
    let paint_res = pipeline.paint(node, &mut ctx);
    assert!(paint_res.is_some());
    let (_buf, pos) = paint_res.unwrap();
    assert_eq!(pos, Position::new(0, 0));
}

#[test]
fn test_render_pipeline_hidpi_paint_allocates_scaled_buffer() {
    let mut pipeline = RenderPipeline::new();
    let mon = MonitorId::new("DP-1");
    let node = RenderNode::Rect {
        path: crate::features::vdom::domain::NodePath::root_in(
            crate::features::vdom::domain::SurfaceSpace::Bar,
        ),
        node_key: None,
        rect: Rect::new(Position::new(10, 20), Size::new(100, 30)),
        style: crate::features::styling::domain::ComputedStyle::default(),
        on_click: None,
        on_hover: None,
        tooltip: None,
        popup: None,
        panel: None,
    };
    let mut fixture = PipelineFixture::<MockLayoutEngine>::default();
    let mut ctx = fixture.ctx(&mon);
    ctx.current_bounds = Some(Rect::new(Position::new(10, 20), Size::new(100, 30)));
    ctx.scale = Scale::new(2.0);
    let paint_res = pipeline.paint(&node, &mut ctx);
    assert!(paint_res.is_some());
    let (buf, pos) = paint_res.unwrap();
    assert_eq!(pos, Position::new(10, 20));
    assert_eq!(buf.width(), 200);
    assert_eq!(buf.height(), 60);
    assert_eq!(buf.data().len(), 200 * 60 * 4);
}

#[test]
fn test_process_monitor_initial_render_and_size_change() {
    let mut pipeline = RenderPipeline::new();
    let mon = MonitorId::new("DP-1");
    let rect = VNode::new_rect(ElementSelectors::default(), Interactions::default());
    let port = TestModulePort::new(rect);
    let resolver = MockStyleResolver;
    let mut factory = MockCanvasFactory;
    let mut engine = MockLayoutEngine;
    let mut popup_engine = MockLayoutEngine;
    let mut panel_engine = MockLayoutEngine;
    let mut tooltip_engine = MockLayoutEngine;
    let child_errors = HashMap::new();

    let ctx = LayoutContext {
        monitor_id: &mon,
        scale: Scale::new(1.0),
        style_resolver: &resolver,
        child_errors: &child_errors,
        current_bounds: None,
        current_constraint: crate::shared::primitives::SizeConstraint::none(),
        current_child_sizes: None,
        interaction_context: None,
        canvas_factory: &mut factory,
        layout_engine: &mut engine,
        popup_layout_engine: &mut popup_engine,
        panel_layout_engine: &mut panel_engine,
        tooltip_layout_engine: &mut tooltip_engine,
    };
    let outcome = process_monitor(&mut pipeline, &port, ctx);

    assert!(outcome.is_some());
    let outcome = outcome.unwrap();
    assert_eq!(
        outcome.size_change(),
        Some(&SizeChange::new(Size::new(0, 0), Size::new(10, 10)))
    );
    assert!(outcome.buffer().is_none());
    assert!(pipeline.render_trees().contains_key(&mon));
    assert!(pipeline.vdom_trees().contains_key(&mon));
}

#[test]
fn test_process_monitor_unchanged_returns_none() {
    let mut pipeline = RenderPipeline::new();
    let mon = MonitorId::new("DP-1");
    let rect = VNode::new_rect(ElementSelectors::default(), Interactions::default());
    let port = TestModulePort::new(rect);
    let resolver = MockStyleResolver;
    let mut factory = MockCanvasFactory;
    let mut engine = MockLayoutEngine;
    let mut popup_engine = MockLayoutEngine;
    let mut panel_engine = MockLayoutEngine;
    let mut tooltip_engine = MockLayoutEngine;
    let child_errors = HashMap::new();

    let ctx1 = LayoutContext {
        monitor_id: &mon,
        scale: Scale::new(1.0),
        style_resolver: &resolver,
        child_errors: &child_errors,
        current_bounds: None,
        current_constraint: crate::shared::primitives::SizeConstraint::none(),
        current_child_sizes: None,
        interaction_context: None,
        canvas_factory: &mut factory,
        layout_engine: &mut engine,
        popup_layout_engine: &mut popup_engine,
        panel_layout_engine: &mut panel_engine,
        tooltip_layout_engine: &mut tooltip_engine,
    };
    let outcome1 = process_monitor(&mut pipeline, &port, ctx1);
    assert!(outcome1.is_some());

    let ctx2 = LayoutContext {
        monitor_id: &mon,
        scale: Scale::new(1.0),
        style_resolver: &resolver,
        child_errors: &child_errors,
        current_bounds: None,
        current_constraint: crate::shared::primitives::SizeConstraint::none(),
        current_child_sizes: None,
        interaction_context: None,
        canvas_factory: &mut factory,
        layout_engine: &mut engine,
        popup_layout_engine: &mut popup_engine,
        panel_layout_engine: &mut panel_engine,
        tooltip_layout_engine: &mut tooltip_engine,
    };
    let outcome2 = process_monitor(&mut pipeline, &port, ctx2);
    assert!(outcome2.is_none());
}
