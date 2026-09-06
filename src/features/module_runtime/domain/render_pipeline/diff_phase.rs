use super::context::PipelineDiff;
use super::pipeline::RenderPipeline;
use crate::features::module_runtime::ports::AnyModulePort;
use crate::features::vdom::domain::InteractionContext;
use crate::features::vdom::ports::VdomDiffPort;
use crate::shared::primitives::geometry::Rect;
use crate::shared::primitives::{ChildSizesMap, MonitorId};

#[must_use]
pub fn diff_pipeline(
    pipeline: &RenderPipeline,
    monitor_id: &MonitorId,
    port: &dyn AnyModulePort,
    vdom_diff: &dyn VdomDiffPort,
    current_bounds: Option<Rect>,
    current_child_sizes: Option<&ChildSizesMap>,
    interaction: Option<&InteractionContext>,
) -> Option<PipelineDiff> {
    let new_vdom = port.render(monitor_id);
    let diff_result = vdom_diff.diff(pipeline.vdom_trees().get(monitor_id), &new_vdom);

    let current_child_sizes_owned = current_child_sizes.cloned();
    let bounds_changed = current_bounds != pipeline.rendered_bounds().get(monitor_id).copied();
    let child_sizes_changed =
        pipeline.last_child_sizes().get(monitor_id) != Some(&current_child_sizes_owned);
    let interaction_changed =
        pipeline.last_interactions().get(monitor_id).map(Option::as_ref) != Some(interaction);

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

    Some(PipelineDiff::new(
        new_vdom,
        vdom_dirty,
        bounds_changed,
        child_sizes_changed,
    ))
}
