use super::context::{LayoutContext, PipelineDiff};
use super::diff_phase::diff_pipeline;
use super::floating_phase::FloatingLayouts;
use super::layout_phase::layout_pipeline;
use super::outcome::{RenderOutcome, SizeChange};
use super::paint_phase::paint_pipeline;
use crate::features::layout_engine::domain::RenderNode;
use crate::features::module_runtime::ports::AnyModulePort;
use crate::features::vdom::domain::{InteractionContext, VNode};
use crate::features::vdom::ports::VdomDiffPort;
use crate::shared::primitives::geometry::{Position, Rect, Scale, Size};
use crate::shared::primitives::render::RenderBuffer;
use crate::shared::primitives::{ChildModuleLayout, ChildSizesMap, MonitorId};
use crate::shared::rendering::ports::canvas::CanvasFactory;
use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct RenderPipeline {
    sizes: HashMap<MonitorId, Size>,
    rendered_bounds: HashMap<MonitorId, Rect>,
    render_trees: HashMap<MonitorId, RenderNode>,
    vdom_trees: HashMap<MonitorId, VNode>,
    last_child_sizes: HashMap<MonitorId, Option<ChildSizesMap>>,
    last_interactions: HashMap<MonitorId, Option<InteractionContext>>,
}

impl RenderPipeline {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub const fn sizes(&self) -> &HashMap<MonitorId, Size> {
        &self.sizes
    }

    pub const fn sizes_mut(&mut self) -> &mut HashMap<MonitorId, Size> {
        &mut self.sizes
    }

    #[must_use]
    pub const fn rendered_bounds(&self) -> &HashMap<MonitorId, Rect> {
        &self.rendered_bounds
    }

    #[must_use]
    pub const fn render_trees(&self) -> &HashMap<MonitorId, RenderNode> {
        &self.render_trees
    }

    pub const fn render_trees_mut(&mut self) -> &mut HashMap<MonitorId, RenderNode> {
        &mut self.render_trees
    }

    #[must_use]
    pub const fn vdom_trees(&self) -> &HashMap<MonitorId, VNode> {
        &self.vdom_trees
    }

    pub const fn vdom_trees_mut(&mut self) -> &mut HashMap<MonitorId, VNode> {
        &mut self.vdom_trees
    }

    #[must_use]
    pub const fn last_child_sizes(&self) -> &HashMap<MonitorId, Option<ChildSizesMap>> {
        &self.last_child_sizes
    }

    pub const fn last_child_sizes_mut(&mut self) -> &mut HashMap<MonitorId, Option<ChildSizesMap>> {
        &mut self.last_child_sizes
    }

    #[must_use]
    pub const fn last_interactions(&self) -> &HashMap<MonitorId, Option<InteractionContext>> {
        &self.last_interactions
    }

    pub const fn last_interactions_mut(
        &mut self,
    ) -> &mut HashMap<MonitorId, Option<InteractionContext>> {
        &mut self.last_interactions
    }

    #[must_use]
    pub fn diff(
        &self,
        monitor_id: &MonitorId,
        port: &dyn AnyModulePort,
        vdom_diff: &dyn VdomDiffPort,
        current_bounds: Option<Rect>,
        current_child_sizes: Option<&ChildSizesMap>,
        interaction: Option<&InteractionContext>,
    ) -> Option<PipelineDiff> {
        diff_pipeline(
            self,
            monitor_id,
            port,
            vdom_diff,
            current_bounds,
            current_child_sizes,
            interaction,
        )
    }

    pub fn layout<F: CanvasFactory>(
        &mut self,
        monitor_id: &MonitorId,
        diff: PipelineDiff,
        ctx: &mut LayoutContext<'_, F>,
    ) -> Option<(RenderNode, Option<SizeChange>, Vec<ChildModuleLayout>, FloatingLayouts)> {
        layout_pipeline(self, monitor_id, diff, ctx)
    }

    pub fn paint<F: CanvasFactory>(
        &mut self,
        monitor_id: &MonitorId,
        render_node: &RenderNode,
        current_bounds: Option<Rect>,
        scale: Scale,
        canvas_factory: &mut F,
    ) -> Option<(RenderBuffer, Position)> {
        let res = paint_pipeline(render_node, current_bounds, scale, canvas_factory);
        if let Some(bounds) = current_bounds.filter(|b| b.width() > 0 && b.height() > 0) {
            self.rendered_bounds.insert(monitor_id.clone(), bounds);
        }
        res
    }

    pub fn process_monitor<F: CanvasFactory>(
        &mut self,
        monitor_id: &MonitorId,
        port: &dyn AnyModulePort,
        vdom_diff: &dyn VdomDiffPort,
        mut ctx: LayoutContext<'_, F>,
    ) -> Option<RenderOutcome> {
        let diff = self.diff(
            monitor_id,
            port,
            vdom_diff,
            ctx.current_bounds,
            ctx.current_child_sizes,
            ctx.interaction_context.as_ref(),
        )?;

        let current_bounds = ctx.current_bounds;
        let (render_node, size_change, child_layouts, floating) =
            self.layout(monitor_id, diff, &mut ctx)?;

        let buffer = self.paint(
            monitor_id,
            &render_node,
            current_bounds,
            ctx.scale,
            ctx.canvas_factory,
        );

        Some(RenderOutcome::new(
            size_change,
            child_layouts,
            render_node,
            buffer,
            floating,
        ))
    }
}
