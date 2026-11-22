//! First pipeline phase: renders the script and decides whether anything changed enough to need a
//! new layout.
use super::context::{DiffInput, PipelineDiff};
use super::pipeline::RenderPipeline;
use crate::features::module_runtime::domain::Change;
use crate::features::module_runtime::ports::AnyModulePort;

#[must_use]
pub(crate) fn diff_pipeline(
    pipeline: &RenderPipeline,
    port: &dyn AnyModulePort,
    input: DiffInput<'_>,
) -> Option<PipelineDiff> {
    let DiffInput {
        monitor_id,
        vdom_diff,
        bounds: current_bounds,
        child_sizes: current_child_sizes,
        interaction,
    } = input;
    let new_vdom = port.render(monitor_id);
    let diff_result = vdom_diff.diff(pipeline.vdom_trees().get(monitor_id), &new_vdom);

    let current_child_sizes_owned = current_child_sizes.cloned();
    let bounds_changed = current_bounds != pipeline.rendered_bounds().get(monitor_id).copied();
    let child_sizes_changed =
        pipeline.last_child_sizes().get(monitor_id) != Some(&current_child_sizes_owned);
    let interaction_changed = pipeline
        .last_interactions()
        .get(monitor_id)
        .map(Option::as_ref)
        != Some(interaction);

    if diff_result.is_unchanged()
        && !bounds_changed
        && !child_sizes_changed
        && !interaction_changed
        && pipeline.render_trees().contains_key(monitor_id)
    {
        tracing::trace!(
            monitor = %monitor_id,
            "VDOM, bounds, child sizes, and interaction unchanged; skipping style resolution, layout, and canvas render"
        );
        return None;
    }

    let vdom_dirty = !diff_result.is_unchanged()
        || interaction_changed
        || !pipeline.render_trees().contains_key(monitor_id);

    Some(PipelineDiff {
        new_vdom,
        vdom: Change::from(vdom_dirty),
        bounds: Change::from(bounds_changed),
        child_sizes: Change::from(child_sizes_changed),
    })
}
