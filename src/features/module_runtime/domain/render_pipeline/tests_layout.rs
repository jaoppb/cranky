//! What the layout phase records, reuses and logs, and what the diff phase logs when it skips.
use super::context::PipelineDiff;
use super::pipeline::RenderPipeline;
use crate::features::layout_engine::domain::{LayoutError, LayoutRoot, RenderNode, TextMeasurer};
use crate::features::layout_engine::ports::LayoutEnginePort;
use crate::features::module_runtime::domain::Change;
use crate::features::module_runtime::test_support::{
    MockLayoutEngine, PipelineFixture, TestModulePort, capture_logs, process_monitor,
};
use crate::features::vdom::domain::{ElementSelectors, Interactions, TextContent, VNode};
use crate::shared::primitives::MonitorId;

const TARGET: &str = "cranky::features::module_runtime::domain::render_pipeline";

fn rect() -> VNode {
    VNode::new_rect(ElementSelectors::default(), Interactions::default())
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

/// A failed layout is logged, yet the tree, child sizes and interaction it was given are already
/// recorded as the last ones of the monitor.
#[test]
fn a_failed_layout_is_logged_after_its_inputs_are_recorded() {
    let mut pipeline = RenderPipeline::new();
    let mut fixture = PipelineFixture::<FailingEngine>::default();
    let mon = MonitorId::new("DP-1");
    let tree = rect();
    let port = TestModulePort::new(tree.clone());
    let logs = capture_logs(|| {
        assert!(process_monitor(&mut pipeline, &port, fixture.ctx(&mon)).is_none());
    });
    assert_eq!(pipeline.vdom_trees().get(&mon), Some(&tree));
    assert_eq!(pipeline.last_child_sizes().get(&mon), Some(&None));
    assert_eq!(pipeline.last_interactions().get(&mon), Some(&None));
    assert!(logs.contains(&format!(
        "ERROR {TARGET}::layout_phase: Module layout calculation failed monitor=DP-1 \
         err=EngineError(\"no layout\")"
    )));
}

/// A diff of a new text that finds nothing dirty.
fn clean_diff() -> PipelineDiff {
    PipelineDiff {
        new_vdom: VNode::new_text(
            TextContent::new("new".to_string()),
            ElementSelectors::default(),
            Interactions::default(),
        ),
        vdom: Change::Unchanged,
        bounds: Change::Unchanged,
        child_sizes: Change::Unchanged,
    }
}

/// A diff with nothing dirty lays nothing out again: the last tree is reused with no size change
/// or child layouts and the new tree is not recorded; with no last tree there is no layout.
#[test]
fn a_diff_with_nothing_dirty_reuses_the_last_tree() {
    let mut pipeline = RenderPipeline::new();
    let mut fixture = PipelineFixture::<MockLayoutEngine>::default();
    let mon = MonitorId::new("DP-1");
    let without_tree = pipeline.layout(clean_diff(), &mut fixture.ctx(&mon));
    assert!(without_tree.is_none());

    let tree = rect();
    let first = process_monitor(
        &mut pipeline,
        &TestModulePort::new(tree.clone()),
        fixture.ctx(&mon),
    );
    let first_tree = first.expect("first render").render_tree().clone();
    let reused = pipeline
        .layout(clean_diff(), &mut fixture.ctx(&mon))
        .expect("reused");
    assert_eq!(reused.render_tree(), &first_tree);
    assert_eq!(reused.report().size_change(), None);
    assert_eq!(reused.report().child_layouts(), []);
    assert_eq!(pipeline.vdom_trees().get(&mon), Some(&tree));
}

/// A layout traces what was dirty, then the style resolution, then the size change; an unchanged
/// render traces that it skipped every phase.
#[test]
fn a_layout_traces_its_steps_and_an_unchanged_render_its_skip() {
    let mut pipeline = RenderPipeline::new();
    let mut fixture = PipelineFixture::<MockLayoutEngine>::default();
    let mon = MonitorId::new("DP-1");
    let port = TestModulePort::new(rect());
    let logs = capture_logs(|| {
        assert!(process_monitor(&mut pipeline, &port, fixture.ctx(&mon)).is_some());
        assert!(process_monitor(&mut pipeline, &port, fixture.ctx(&mon)).is_none());
    });
    let expected = [
        "TRACE {T}::layout_phase: Updating layout for module monitor=DP-1 vdom_dirty=true \
         bounds_changed=false child_sizes_changed=true",
        "TRACE {T}::layout_phase: Resolving styles for module VNode monitor=DP-1",
        "TRACE {T}::layout_phase: Module size changed monitor=DP-1 size=Size { width: 10, height: \
         10 } old_size=Size { width: 0, height: 0 } settled_width=None",
        "TRACE {T}::diff_phase: VDOM, bounds, child sizes, and interaction unchanged; skipping \
         style resolution, layout, and canvas render monitor=DP-1",
    ]
    .map(|line| line.replace("{T}", TARGET));
    let lines: Vec<&str> = logs.lines().collect();
    assert_eq!(lines, expected);
}
