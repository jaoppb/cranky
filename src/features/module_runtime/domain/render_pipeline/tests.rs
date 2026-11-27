use super::outcome::SizeChange;
use super::pipeline::RenderPipeline;
use crate::features::layout_engine::domain::RenderNode;
use crate::features::module_runtime::test_support::{
    MockLayoutEngine, MockVdomDiff, PipelineFixture, TestModulePort, process_monitor,
};
use crate::features::vdom::domain::{ElementSelectors, Interactions, VNode};
use crate::shared::primitives::MonitorId;
use crate::shared::primitives::geometry::{Position, Rect, Scale, Size};

fn rect_port() -> TestModulePort {
    TestModulePort::new(VNode::new_rect(
        ElementSelectors::default(),
        Interactions::default(),
    ))
}

/// The diff of a first render is dirty, and its layout lays out the mock engine's 10x10 tree with
/// nothing floating, no child layouts, and a size change from nothing.
#[test]
fn test_pipeline_diff_and_layout_phases() {
    let mut pipeline = RenderPipeline::new();
    let mon = MonitorId::new("DP-1");
    let mut fixture = PipelineFixture::<MockLayoutEngine>::default();
    let mut ctx = fixture.ctx(&mon);
    // 1. Diff Phase
    let diff = pipeline
        .diff(&rect_port(), ctx.diff_input(&MockVdomDiff))
        .expect("a first render diffs");
    assert!(diff.vdom_dirty());

    // 2. Layout Phase
    let layout = pipeline
        .layout(diff, &mut ctx)
        .expect("a dirty diff lays out");
    let node = layout.render_tree();
    assert_eq!(
        (layout.floating().popup(), layout.floating().panel()),
        (None, None)
    );
    let grown = SizeChange::new(Size::new(0, 0), Size::new(10, 10));
    assert_eq!(layout.report().size_change(), Some(&grown));
    assert_eq!(layout.report().child_layouts(), []);
    assert_eq!(*node.rect().size(), Size::new(10, 10));
}

/// Painting without bounds paints nothing; with bounds it paints at their position.
#[test]
fn test_pipeline_paint_phase_needs_bounds() {
    let mut pipeline = RenderPipeline::new();
    let mon = MonitorId::new("DP-1");
    let mut fixture = PipelineFixture::<MockLayoutEngine>::default();
    let outcome = process_monitor(&mut pipeline, &rect_port(), fixture.ctx(&mon));
    let node = outcome.expect("first render").render_tree().clone();
    let mut ctx = fixture.ctx(&mon);

    // 3. Paint Phase without bounds returns None
    let paint_res = pipeline.paint(&node, &mut ctx);
    assert!(paint_res.is_none());

    // 4. Paint Phase with valid bounds returns buffer
    ctx.current_bounds = Some(Rect::new(Position::new(0, 0), Size::new(10, 10)));
    let paint_res = pipeline.paint(&node, &mut ctx);
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
    let (buf, pos) = pipeline.paint(&node, &mut ctx).expect("painted");
    assert_eq!(pos, Position::new(10, 20));
    assert_eq!(buf.width(), 200);
    assert_eq!(buf.height(), 60);
    assert_eq!(buf.data().len(), 200 * 60 * 4);
}

#[test]
fn test_process_monitor_initial_render_and_size_change() {
    let mut pipeline = RenderPipeline::new();
    let mon = MonitorId::new("DP-1");
    let mut fixture = PipelineFixture::<MockLayoutEngine>::default();
    let outcome = process_monitor(&mut pipeline, &rect_port(), fixture.ctx(&mon));

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
    let port = rect_port();
    let mut fixture = PipelineFixture::<MockLayoutEngine>::default();

    let outcome1 = process_monitor(&mut pipeline, &port, fixture.ctx(&mon));
    assert!(outcome1.is_some());

    let outcome2 = process_monitor(&mut pipeline, &port, fixture.ctx(&mon));
    assert!(outcome2.is_none());
}
