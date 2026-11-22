//! The floating part of the layout phase: lays out the popup, panel and tooltip content open in
//! the module's tree, each kind with its own engine and cache.
use super::super::context::LayoutContext;
use super::super::floating_phase::{
    FloatingEngine, FloatingEngines, FloatingLayouts, FloatingPass, layout_floating,
};
use super::super::pipeline::RenderPipeline;
use crate::features::layout_engine::domain::RenderNode;
use crate::features::layout_engine::ports::LayoutEnginePort;
use crate::shared::rendering::ports::canvas::CanvasFactory;

/// Lays out the floating content open in `render_node`, each kind with its own engine and cache.
pub(super) fn floating_layouts<F: CanvasFactory, E: LayoutEnginePort>(
    pipeline: &mut RenderPipeline,
    render_node: &RenderNode,
    ctx: &mut LayoutContext<'_, F, E>,
) -> FloatingLayouts {
    let caches = pipeline.floating_content_caches_mut();
    let engines = FloatingEngines {
        popup: FloatingEngine::new(ctx.popup_layout_engine, caches.popup),
        panel: FloatingEngine::new(ctx.panel_layout_engine, caches.panel),
        tooltip: FloatingEngine::new(ctx.tooltip_layout_engine, caches.tooltip),
    };
    layout_floating(
        render_node,
        engines,
        &mut FloatingPass {
            monitor_id: ctx.monitor_id,
            canvas_factory: ctx.canvas_factory,
            scale: ctx.scale,
            current_child_sizes: ctx.current_child_sizes,
            interaction: ctx.interaction_context.as_ref(),
        },
    )
}
