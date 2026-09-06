use super::context::LayoutContext;
use super::outcome::SizeChange;
use super::pipeline::RenderPipeline;
use crate::features::layout_engine::domain::RenderNode;
use crate::features::module_runtime::test_support::{
    MockCanvasFactory, MockLayoutEngine, MockStyleResolver, MockVdomDiff, TestModulePort,
};
use crate::features::vdom::domain::VNode;
use crate::shared::primitives::geometry::{Position, Rect, Scale, Size};
use crate::shared::primitives::MonitorId;

#[test]
fn test_pipeline_diff_and_layout_phases() {
    let mut pipeline = RenderPipeline::new();
    let mon = MonitorId::new("DP-1");
    let port = TestModulePort::new(VNode::new_rect(None, None, None, None, None));
    let diff_adapter = MockVdomDiff;
    let resolver = MockStyleResolver;
    let mut factory = MockCanvasFactory;
    let mut engine = MockLayoutEngine;

    // 1. Diff Phase
    let diff = pipeline.diff(&mon, &port, &diff_adapter, None, None, None);
    assert!(diff.is_some());
    let diff = diff.unwrap();
    assert!(diff.vdom_dirty());

    // 2. Layout Phase
    let mut ctx = LayoutContext {
        scale: Scale::new(1.0),
        style_resolver: &resolver,
        current_bounds: None,
        current_child_sizes: None,
        interaction_context: None,
        canvas_factory: &mut factory,
        layout_engine: &mut engine,
    };
    let layout_res = pipeline.layout(&mon, diff, &mut ctx);
    assert!(layout_res.is_some());
    let (node, size_change, child_layouts) = layout_res.unwrap();
    assert_eq!(
        size_change,
        Some(SizeChange::new(Size::new(0, 0), Size::new(10, 10)))
    );
    assert!(child_layouts.is_empty());
    assert_eq!(*node.rect().size(), Size::new(10, 10));

    // 3. Paint Phase without bounds returns None
    let paint_res = pipeline.paint(&mon, &node, None, Scale::new(1.0), &mut factory);
    assert!(paint_res.is_none());

    // 4. Paint Phase with valid bounds returns buffer
    let bounds = Rect::new(Position::new(0, 0), Size::new(10, 10));
    let paint_res = pipeline.paint(&mon, &node, Some(bounds), Scale::new(1.0), &mut factory);
    assert!(paint_res.is_some());
    let (_buf, pos) = paint_res.unwrap();
    assert_eq!(pos, Position::new(0, 0));
}

#[test]
fn test_render_pipeline_hidpi_paint_allocates_scaled_buffer() {
    let mut pipeline = RenderPipeline::new();
    let mon = MonitorId::new("DP-1");
    let node = RenderNode::Rect {
        path: crate::features::vdom::domain::NodePath::root(),
        rect: Rect::new(Position::new(10, 20), Size::new(100, 30)),
        style: crate::features::styling::domain::ComputedStyle::default(),
        on_click: None,
        on_hover: None,
        tooltip: None,
        popup: None,
    };
    let mut factory = MockCanvasFactory;

    let bounds = Rect::new(Position::new(10, 20), Size::new(100, 30));
    let paint_res = pipeline.paint(&mon, &node, Some(bounds), Scale::new(2.0), &mut factory);
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
    let port = TestModulePort::new(VNode::new_rect(None, None, None, None, None));
    let diff = MockVdomDiff;
    let resolver = MockStyleResolver;
    let mut factory = MockCanvasFactory;
    let mut engine = MockLayoutEngine;

    let ctx = LayoutContext {
        scale: Scale::new(1.0),
        style_resolver: &resolver,
        current_bounds: None,
        current_child_sizes: None,
        interaction_context: None,
        canvas_factory: &mut factory,
        layout_engine: &mut engine,
    };
    let outcome = pipeline.process_monitor(&mon, &port, &diff, ctx);

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
    let port = TestModulePort::new(VNode::new_rect(None, None, None, None, None));
    let diff = MockVdomDiff;
    let resolver = MockStyleResolver;
    let mut factory = MockCanvasFactory;
    let mut engine = MockLayoutEngine;

    let ctx1 = LayoutContext {
        scale: Scale::new(1.0),
        style_resolver: &resolver,
        current_bounds: None,
        current_child_sizes: None,
        interaction_context: None,
        canvas_factory: &mut factory,
        layout_engine: &mut engine,
    };
    let outcome1 = pipeline.process_monitor(&mon, &port, &diff, ctx1);
    assert!(outcome1.is_some());

    let ctx2 = LayoutContext {
        scale: Scale::new(1.0),
        style_resolver: &resolver,
        current_bounds: None,
        current_child_sizes: None,
        interaction_context: None,
        canvas_factory: &mut factory,
        layout_engine: &mut engine,
    };
    let outcome2 = pipeline.process_monitor(&mon, &port, &diff, ctx2);
    assert!(outcome2.is_none());
}
