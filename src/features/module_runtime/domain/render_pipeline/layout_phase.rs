use super::context::{LayoutContext, PipelineDiff};
use super::measurer::ModuleSizeMeasurer;
use super::outcome::SizeChange;
use super::pipeline::RenderPipeline;
use crate::features::layout_engine::domain::RenderNode;
use crate::shared::config::domain::{FontFamily, FontSize};
use crate::shared::primitives::geometry::{Position, Size};
use crate::shared::primitives::{ChildModuleLayout, MonitorId};
use crate::shared::rendering::ports::canvas::CanvasFactory;

#[allow(clippy::needless_pass_by_value)]
pub fn layout_pipeline<F: CanvasFactory>(
    pipeline: &mut RenderPipeline,
    monitor_id: &MonitorId,
    diff: PipelineDiff,
    ctx: &mut LayoutContext<'_, F>,
) -> Option<(RenderNode, Option<SizeChange>, Vec<ChildModuleLayout>)> {
    let current_child_sizes_owned = ctx.current_child_sizes.cloned();
    let mut size_change = None;
    let mut child_layouts = Vec::new();

    let render_node = if diff.vdom_dirty() || diff.bounds_changed() || diff.child_sizes_changed() {
        pipeline
            .last_child_sizes_mut()
            .insert(monitor_id.clone(), current_child_sizes_owned);
        pipeline
            .last_interactions_mut()
            .insert(monitor_id.clone(), ctx.interaction_context.clone());
        tracing::trace!(
            monitor = %monitor_id,
            vdom_dirty = diff.vdom_dirty(),
            bounds_changed = diff.bounds_changed(),
            child_sizes_changed = diff.child_sizes_changed(),
            "Updating layout for module"
        );

        pipeline
            .vdom_trees_mut()
            .insert(monitor_id.clone(), diff.new_vdom().clone());

        tracing::trace!(monitor = %monitor_id, "Resolving styles for module VNode");
        let styled_node = diff.new_vdom().resolve_styles(
            ctx.style_resolver,
            ctx.interaction_context.as_ref(),
            None,
        );

        let default_font_family = FontFamily::new(String::new());
        let default_font_size = FontSize::new(14.0);

        let available_size = ctx
            .current_bounds
            .filter(|b| b.width() > 0 && b.height() > 0)
            .map(|b| *b.size());

        let measurer_inner = ctx.canvas_factory.create_text_measurer(
            ctx.scale,
            default_font_family,
            default_font_size,
        );
        let mut measurer = ModuleSizeMeasurer::new(measurer_inner, ctx.current_child_sizes);

        let render_node_res = ctx.layout_engine.calculate_layout_with_constraints(
            styled_node,
            &mut measurer,
            Position::new(0, 0),
            available_size,
        );

        let render_node = match render_node_res {
            Ok(node) => node,
            Err(e) => {
                tracing::error!(monitor = %monitor_id, err = ?e, "Module layout calculation failed");
                return None;
            }
        };

        child_layouts = render_node.collect_module_layouts();

        pipeline
            .render_trees_mut()
            .insert(monitor_id.clone(), render_node.clone());

        let size = *render_node.rect().size();
        let old_size = pipeline
            .sizes()
            .get(monitor_id)
            .copied()
            .unwrap_or(Size::new(0, 0));

        if size != old_size {
            pipeline.sizes_mut().insert(monitor_id.clone(), size);
            tracing::trace!(
                monitor = %monitor_id,
                ?size,
                ?old_size,
                "Module size changed"
            );
            size_change = Some(SizeChange::new(old_size, size));
        }

        render_node
    } else {
        pipeline.render_trees().get(monitor_id)?.clone()
    };

    Some((render_node, size_change, child_layouts))
}
