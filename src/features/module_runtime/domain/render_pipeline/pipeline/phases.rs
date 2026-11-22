use super::RenderPipeline;
use crate::features::layout_engine::domain::RenderNode;
use crate::features::layout_engine::ports::LayoutEnginePort;
use crate::features::module_runtime::domain::render_pipeline::context::{
    DiffInput, LayoutContext, PipelineDiff,
};
use crate::features::module_runtime::domain::render_pipeline::diff_phase::diff_pipeline;
use crate::features::module_runtime::domain::render_pipeline::layout_phase::layout_pipeline;
use crate::features::module_runtime::domain::render_pipeline::outcome::{
    PipelineLayout, RenderOutcome,
};
use crate::features::module_runtime::domain::render_pipeline::paint_phase::{
    ScaledCanvasFactory, paint_pipeline,
};
use crate::features::module_runtime::ports::AnyModulePort;
use crate::shared::primitives::geometry::Position;
use crate::shared::primitives::render::RenderBuffer;
use crate::shared::rendering::ports::canvas::CanvasFactory;

impl RenderPipeline {
    /// Renders the module for the input's monitor and returns what changed, or `None` when the
    /// tree, bounds, child sizes and interaction all match the last render on that monitor.
    #[must_use]
    pub fn diff(&self, port: &dyn AnyModulePort, input: DiffInput<'_>) -> Option<PipelineDiff> {
        diff_pipeline(self, port, input)
    }

    /// Lays out the diffed tree and its floating content on the context's monitor, returning the
    /// render tree, the floating layouts and the size change and child module layouts to report.
    ///
    /// Returns `None` when the layout engine fails, or when nothing is dirty and no earlier tree
    /// exists.
    pub fn layout<F: CanvasFactory, E: LayoutEnginePort>(
        &mut self,
        diff: PipelineDiff,
        ctx: &mut LayoutContext<'_, F, E>,
    ) -> Option<PipelineLayout> {
        layout_pipeline(self, diff, ctx)
    }

    pub(crate) fn paint<F: CanvasFactory, E: LayoutEnginePort>(
        &mut self,
        render_node: &RenderNode,
        ctx: &mut LayoutContext<'_, F, E>,
    ) -> Option<(RenderBuffer, Position)> {
        let current_bounds = ctx.current_bounds;
        let canvases = ScaledCanvasFactory::new(ctx.canvas_factory, ctx.scale);
        let res = paint_pipeline(render_node, current_bounds, canvases);
        if let Some(bounds) = current_bounds.filter(|rect| rect.width() > 0 && rect.height() > 0) {
            self.rendered_bounds.insert(ctx.monitor_id.clone(), bounds);
        }
        res
    }

    /// Lays out and paints what the diff phase found on the context's monitor, or `None` when the
    /// layout phase produced nothing.
    pub(crate) fn layout_and_paint<F: CanvasFactory, E: LayoutEnginePort>(
        &mut self,
        diff: PipelineDiff,
        mut ctx: LayoutContext<'_, F, E>,
    ) -> Option<RenderOutcome> {
        let layout = self.layout(diff, &mut ctx)?;
        let buffer = self.paint(layout.render_tree(), &mut ctx);
        Some(RenderOutcome::new(layout, buffer, ctx.scale))
    }
}
