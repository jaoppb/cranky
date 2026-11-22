//! How each per-monitor input of a render reaches the phases: the interaction, the monitor the
//! state is kept under and the painted bounds.
use super::outcome::SizeChange;
use super::pipeline::RenderPipeline;
use crate::features::layout_engine::domain::LayoutRoot;
use crate::features::layout_engine::domain::{LayoutError, RenderNode, TextMeasurer};
use crate::features::layout_engine::ports::LayoutEnginePort;
use crate::features::module_runtime::test_support::{
    MockLayoutEngine, MockVdomDiff, PipelineFixture, TestModulePort, process_monitor,
};
use crate::features::vdom::domain::{
    InteractedNodes, InteractionContext, NodePath, NodeRef, SurfaceSpace, VNode,
};
use crate::shared::primitives::geometry::{Position, Rect, Scale, Size};
use crate::shared::primitives::{MonitorFocus, MonitorId};

fn rect() -> VNode {
    VNode::new_rect(None, None, None, None, None)
}

/// A new interaction alone dirties the tree (pseudo-classes restyle it) and nothing else.
#[test]
fn an_interaction_alone_dirties_the_tree_only() {
    let mut pipeline = RenderPipeline::new();
    let mut fixture = PipelineFixture::<MockLayoutEngine>::default();
    let mon = MonitorId::new("DP-1");
    let port = TestModulePort::new(rect());
    assert!(process_monitor(&mut pipeline, &port, fixture.ctx(&mon)).is_some());

    let hovered = NodeRef::new(Vec::new(), NodePath::root_in(SurfaceSpace::Bar));
    let interaction = InteractionContext::new(
        InteractedNodes::new(Some(hovered), None, None),
        MonitorFocus::Focused,
    );
    let mut ctx = fixture.ctx(&mon);
    ctx.interaction_context = Some(interaction);
    let diff = pipeline
        .diff(&port, ctx.diff_input(&MockVdomDiff))
        .expect("a new hover is a change");
    assert!(diff.vdom_dirty());
    assert!(!diff.bounds_changed());
    assert!(!diff.child_sizes_changed());
}

/// Each monitor keeps its own last render and its own reported size.
#[test]
fn each_monitor_is_diffed_and_sized_on_its_own() {
    let mut pipeline = RenderPipeline::new();
    let mut fixture = PipelineFixture::<MockLayoutEngine>::default();
    let (first, second) = (MonitorId::new("DP-1"), MonitorId::new("HDMI-A-1"));
    let port = TestModulePort::new(rect());
    let grown = SizeChange::new(Size::new(0, 0), Size::new(10, 10));

    let outcome = process_monitor(&mut pipeline, &port, fixture.ctx(&first));
    assert_eq!(outcome.expect("first render").size_change(), Some(&grown));
    assert!(
        pipeline
            .diff(&port, fixture.ctx(&first).diff_input(&MockVdomDiff))
            .is_none()
    );

    let outcome = process_monitor(&mut pipeline, &port, fixture.ctx(&second));
    assert_eq!(outcome.expect("second monitor").size_change(), Some(&grown));
    assert!(pipeline.render_trees().contains_key(&second));
}

/// Painting records the bounds it painted at, and nothing for empty bounds.
#[test]
fn paint_records_only_the_bounds_it_painted_at() {
    let mut pipeline = RenderPipeline::new();
    let mut fixture = PipelineFixture::<MockLayoutEngine>::default();
    let mon = MonitorId::new("DP-1");
    let node = process_monitor(
        &mut pipeline,
        &TestModulePort::new(rect()),
        fixture.ctx(&mon),
    )
    .expect("render outcome")
    .render_tree()
    .clone();
    let mut ctx = fixture.ctx(&mon);
    ctx.scale = Scale::new(1.5);

    ctx.current_bounds = Some(Rect::new(Position::new(3, 4), Size::new(20, 0)));
    let painted = pipeline.paint(&node, &mut ctx);
    assert!(painted.is_none());
    assert!(pipeline.rendered_bounds().is_empty());

    let bounds = Rect::new(Position::new(3, 4), Size::new(20, 10));
    ctx.current_bounds = Some(bounds);
    let (buffer, position) = pipeline.paint(&node, &mut ctx).expect("painted");
    assert_eq!((buffer.width(), buffer.height()), (30, 15));
    assert_eq!(position, Position::new(3, 4));
    assert_eq!(pipeline.rendered_bounds().get(&mon), Some(&bounds));
}

/// A layout engine that always fails.
#[derive(Default)]
struct FailingEngine;

impl LayoutEnginePort for FailingEngine {
    fn calculate_layout(
        &mut self,
        _root: LayoutRoot,
        _measurer: &mut dyn TextMeasurer,
    ) -> Result<RenderNode, LayoutError> {
        Err(LayoutError::EngineError("no layout".to_string()))
    }
}

/// When the layout engine fails, the render produces nothing and keeps no tree for the monitor.
#[test]
fn a_failed_layout_renders_nothing() {
    let mut pipeline = RenderPipeline::new();
    let mut fixture = PipelineFixture::<FailingEngine>::default();
    let mon = MonitorId::new("DP-1");

    let outcome = process_monitor(
        &mut pipeline,
        &TestModulePort::new(rect()),
        fixture.ctx(&mon),
    );

    assert!(outcome.is_none());
    assert!(pipeline.render_trees().is_empty());
}
