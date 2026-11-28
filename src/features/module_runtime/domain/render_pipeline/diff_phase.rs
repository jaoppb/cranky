//! First pipeline phase: renders the script and decides whether anything changed enough to need a
//! new layout.
use super::context::{DiffInput, PipelineDiff};
use super::pipeline::RenderPipeline;
use crate::features::module_runtime::domain::Change;
use crate::features::module_runtime::ports::AnyModulePort;
use crate::features::vdom::domain::VNode;
use crate::shared::primitives::MonitorId;

#[must_use]
pub(crate) fn diff_pipeline(
    pipeline: &RenderPipeline,
    port: &dyn AnyModulePort,
    input: DiffInput<'_>,
) -> Option<PipelineDiff> {
    let monitor_id = input.monitor_id;
    let new_vdom = port.render(monitor_id);
    let diff = PipelineDiff {
        vdom: Change::from(vdom_dirty(pipeline, &input, &new_vdom)),
        bounds: Change::from(bounds_changed(pipeline, &input)),
        child_sizes: Change::from(child_sizes_changed(pipeline, &input)),
        new_vdom,
    };
    if !diff.vdom_dirty() && !diff.bounds_changed() && !diff.child_sizes_changed() {
        log_render_skipped(monitor_id);
        return None;
    }
    Some(diff)
}

/// Whether the tree must be styled and laid out again on the input's monitor: the script's tree
/// differs from the last one, the interaction that restyles it changed, or no tree was laid out
/// there yet.
fn vdom_dirty(pipeline: &RenderPipeline, input: &DiffInput<'_>, new_vdom: &VNode) -> bool {
    let monitor_id = input.monitor_id;
    let diff_result = input
        .vdom_diff
        .diff(pipeline.vdom_trees().get(monitor_id), new_vdom);
    let interaction_changed = pipeline
        .last_interactions()
        .get(monitor_id)
        .map(Option::as_ref)
        != Some(input.interaction);
    !diff_result.is_unchanged()
        || interaction_changed
        || !pipeline.render_trees().contains_key(monitor_id)
}

/// Whether the module is placed differently on the input's monitor than when it was last painted.
fn bounds_changed(pipeline: &RenderPipeline, input: &DiffInput<'_>) -> bool {
    input.bounds != pipeline.rendered_bounds().get(input.monitor_id).copied()
}

/// Whether the sizes the embedded children reported differ from those of the last layout on the
/// input's monitor.
fn child_sizes_changed(pipeline: &RenderPipeline, input: &DiffInput<'_>) -> bool {
    let current_child_sizes_owned = input.child_sizes.cloned();
    pipeline.last_child_sizes().get(input.monitor_id) != Some(&current_child_sizes_owned)
}

/// Logs that nothing changed on `monitor_id`, so no phase after the diff runs.
///
/// Kept out of line to keep its caller under `cognitive_complexity`.
fn log_render_skipped(monitor_id: &MonitorId) {
    tracing::trace!(
        monitor = %monitor_id,
        "VDOM, bounds, child sizes, and interaction unchanged; skipping style resolution, layout, and canvas render"
    );
}
