//! Second pipeline phase: resolves styles, lays out the module's tree and its floating content,
//! and records any size change to report to the embedder.
mod floating;

use super::context::{LayoutContext, PipelineDiff};
use super::floating_phase::apply_gate;
use super::measurer::ModuleSizeMeasurer;
use super::outcome::{EmbedReport, PipelineLayout, SizeChange};
use super::pipeline::RenderPipeline;
use crate::features::layout_engine::domain::{LayoutError, LayoutRoot};
use crate::features::layout_engine::domain::{RenderNode, StyledNode};
use crate::features::layout_engine::ports::LayoutEnginePort;
use crate::features::vdom::domain::VNode;
use crate::shared::primitives::geometry::{Position, Size};
use crate::shared::primitives::{ChildModuleLayout, ChildReport, MonitorId};
use crate::shared::rendering::ports::canvas::CanvasFactory;
use crate::shared::rendering::ports::font::Font;
use floating::floating_layouts;

/// Records what this layout reports to its embedder and returns the change,
/// if any: the unpinned size, not the pinned one (the embedder sizes the host
/// from it, so it must not depend on the width it chose), plus the width the
/// layout settled at.
fn report_size(
    pipeline: &mut RenderPipeline,
    monitor_id: &MonitorId,
    report: ChildReport,
) -> Option<SizeChange> {
    let size = *report.size();
    let settled_width = report.settled_width();
    let old_size = pipeline
        .sizes()
        .get(monitor_id)
        .copied()
        .unwrap_or(Size::new(0, 0));
    let settled_changed = pipeline.embed.settle(monitor_id, settled_width);
    if size == old_size && !settled_changed {
        return None;
    }
    pipeline.sizes_mut().insert(monitor_id.clone(), size);
    log_size_changed(monitor_id, &report, old_size);
    Some(SizeChange::new(old_size, size).with_settled_width(settled_width))
}

/// Logs that the module on `monitor_id` now reports `report`, up from `old_size`.
fn log_size_changed(monitor_id: &MonitorId, report: &ChildReport, old_size: Size) {
    let size = *report.size();
    let settled_width = report.settled_width();
    tracing::trace!(monitor = %monitor_id, ?size, ?old_size, ?settled_width, "Module size changed");
}

/// Records the tree and the child sizes and interaction it is laid out with as the last ones of
/// the context's monitor.
fn record_layout_inputs<F: CanvasFactory, E: LayoutEnginePort>(
    pipeline: &mut RenderPipeline,
    diff: &PipelineDiff,
    ctx: &LayoutContext<'_, F, E>,
) {
    let monitor_id = ctx.monitor_id;
    pipeline
        .last_child_sizes_mut()
        .insert(monitor_id.clone(), ctx.current_child_sizes.cloned());
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
}

/// Resolves the styles of `vdom`, with each failed child module replaced by its placeholder.
fn resolve_styles<F: CanvasFactory, E: LayoutEnginePort>(
    vdom: &VNode,
    ctx: &LayoutContext<'_, F, E>,
) -> StyledNode {
    if ctx.child_errors.is_empty() {
        vdom.resolve_styles(ctx.style_resolver, ctx.interaction_context.as_ref())
    } else {
        let substituted =
            crate::features::vdom::domain::substitute_failed_modules(vdom, ctx.child_errors);
        substituted.resolve_styles(ctx.style_resolver, ctx.interaction_context.as_ref())
    }
}

/// Lays `styled_node` out within the context's constraint; `None`, logged, when the engine fails.
fn calculate_bar_layout<F: CanvasFactory, E: LayoutEnginePort>(
    styled_node: StyledNode,
    ctx: &mut LayoutContext<'_, F, E>,
) -> Option<RenderNode> {
    let measurer_inner = ctx
        .canvas_factory
        .create_text_measurer(ctx.scale, Font::fallback());
    let mut measurer = ModuleSizeMeasurer::new(measurer_inner, ctx.current_child_sizes);

    let render_node_res = ctx.layout_engine.calculate_layout(
        LayoutRoot::new(styled_node, Position::new(0, 0), ctx.current_constraint),
        &mut measurer,
    );

    match render_node_res {
        Ok(node) => Some(node),
        Err(error) => {
            log_layout_failed(ctx.monitor_id, &error);
            None
        }
    }
}

/// Logs that the module's tree on `monitor_id` is about to be styled.
fn log_resolving_styles(monitor_id: &MonitorId) {
    tracing::trace!(monitor = %monitor_id, "Resolving styles for module VNode");
}

/// Logs a layout of the module's tree on `monitor_id` that failed with `error`.
fn log_layout_failed(monitor_id: &MonitorId, error: &LayoutError) {
    tracing::error!(monitor = %monitor_id, err = ?error, "Module layout calculation failed");
}

/// Lays the diffed tree out again and keeps it as the monitor's tree: the tree, its size change
/// and the modules it embeds; `None` when the layout fails.
fn relayout<F: CanvasFactory, E: LayoutEnginePort>(
    pipeline: &mut RenderPipeline,
    diff: &PipelineDiff,
    ctx: &mut LayoutContext<'_, F, E>,
) -> Option<(RenderNode, Option<SizeChange>, Vec<ChildModuleLayout>)> {
    let monitor_id = ctx.monitor_id;
    record_layout_inputs(pipeline, diff, ctx);
    log_resolving_styles(monitor_id);
    let styled_node = resolve_styles(diff.new_vdom(), ctx);
    let render_node = calculate_bar_layout(styled_node, ctx)?;
    let child_layouts = render_node.collect_module_layouts();

    pipeline
        .render_trees_mut()
        .insert(monitor_id.clone(), render_node.clone());

    let size = ctx
        .layout_engine
        .intrinsic_size()
        .unwrap_or_else(|| *render_node.rect().size());
    let report = ChildReport::new(size, ctx.current_constraint.width());
    let size_change = report_size(pipeline, monitor_id, report);
    Some((render_node, size_change, child_layouts))
}

pub(crate) fn layout_pipeline<F: CanvasFactory, E: LayoutEnginePort>(
    pipeline: &mut RenderPipeline,
    diff: PipelineDiff,
    ctx: &mut LayoutContext<'_, F, E>,
) -> Option<PipelineLayout> {
    let monitor_id = ctx.monitor_id;
    let (render_node, size_change, mut child_layouts) =
        if diff.vdom_dirty() || diff.bounds_changed() || diff.child_sizes_changed() {
            relayout(pipeline, &diff, ctx)?
        } else {
            (
                pipeline.render_trees().get(monitor_id)?.clone(),
                None,
                Vec::new(),
            )
        };

    drop(diff);

    let floating = floating_layouts(pipeline, &render_node, ctx);
    child_layouts.extend(floating.module_layouts());

    let rounds = pipeline.embed.gate_rounds_mut().entry(monitor_id.clone());
    let floating = apply_gate(floating, ctx.current_child_sizes, rounds);

    Some(PipelineLayout::new(
        render_node,
        floating,
        EmbedReport::new(size_change, child_layouts),
    ))
}
