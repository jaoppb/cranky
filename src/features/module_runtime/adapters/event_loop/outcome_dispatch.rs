use super::dispatch::{dispatch_outcome_buffer, dispatch_outcome_layouts};
use super::floating_dispatch::{
    dispatch_outcome_panels, dispatch_outcome_popups, dispatch_outcome_tooltips,
};
use super::runner::EventLoop;
use crate::features::layout_engine::domain::DisplayCommandSender;
use crate::features::module_runtime::domain::RenderOutcome;
use crate::features::module_runtime::ports::LayoutEventSender;
use crate::features::vdom::domain::UiCommandSender;
use crate::shared::primitives::MonitorId;
use crate::shared::primitives::geometry::Scale;
use crate::shared::rendering::ports::canvas::CanvasFactory;

impl<
    F: CanvasFactory + 'static,
    LS: LayoutEventSender + 'static,
    DS: DisplayCommandSender + 'static,
    US: UiCommandSender + 'static,
> EventLoop<F, LS, DS, US>
{
    pub fn dispatch_render_outcome(
        &mut self,
        monitor_id: &MonitorId,
        outcome: &RenderOutcome,
        scale: Scale,
    ) {
        dispatch_outcome_layouts(
            self.ctx.layout_sender(),
            self.ctx.id(),
            monitor_id,
            outcome,
            &mut self.child_layouts_present,
        );
        dispatch_outcome_buffer(
            self.ctx.surface_manager(),
            self.ctx.id(),
            self.ctx.parent_id(),
            self.ctx.surface(),
            monitor_id,
            outcome,
        );
        dispatch_outcome_popups(
            self.ctx.display_sender(),
            &mut self.popup_render_trees,
            &mut self.canvas_factory,
            scale,
            self.ctx.id(),
            monitor_id,
            outcome,
        );
        dispatch_outcome_panels(
            self.ctx.display_sender(),
            &mut self.panel_render_trees,
            &mut self.canvas_factory,
            scale,
            self.ctx.id(),
            monitor_id,
            outcome,
        );
        dispatch_outcome_tooltips(
            self.ctx.display_sender(),
            &mut self.tooltip_render_trees,
            &mut self.canvas_factory,
            scale,
            monitor_id,
            outcome,
        );
    }
}
