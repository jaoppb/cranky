use super::dispatch::{dispatch_outcome_buffer, dispatch_outcome_layouts, surface_parent_for};
use super::floating_dispatch::FloatingDispatcher;
use super::runner::EventLoop;
use crate::features::layout_engine::domain::DisplayCommandSender;
use crate::features::module_runtime::domain::RenderOutcome;
use crate::features::module_runtime::ports::LayoutEventSender;
use crate::features::vdom::domain::UiCommandSender;
use crate::shared::primitives::MonitorId;
use crate::shared::rendering::ports::canvas::CanvasFactory;
use crate::shared::wayland::ports::SurfaceParent;

impl<
    F: CanvasFactory + 'static,
    LS: LayoutEventSender + 'static,
    DS: DisplayCommandSender + 'static,
    US: UiCommandSender + 'static,
> EventLoop<F, LS, DS, US>
{
    pub(crate) fn dispatch_render_outcome(
        &mut self,
        monitor_id: &MonitorId,
        outcome: &RenderOutcome,
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
        // The floating surface this module's own popup/panel nests inside
        // (decision 7), if any — the same embedding site
        // `dispatch_outcome_buffer` just used to parent this module's own
        // subsurface, since a module opening its own popup while itself
        // embedded in someone else's is exactly what nests a popup.
        let floating_parent = self.ctx.parent_id().and_then(|pid| {
            match surface_parent_for(pid, self.ctx.surface(), monitor_id) {
                SurfaceParent::Bar => None,
                SurfaceParent::Floating(kind) => Some(kind),
            }
        });
        let module_id = self.ctx.id();
        let mut floating = FloatingDispatcher {
            display_sender: self.ctx.display_sender(),
            canvas_factory: &mut self.canvas_factory,
            scale: outcome.scale(),
            monitor_id,
            parent: floating_parent,
        };
        floating.popups(&mut self.popup_render_trees, module_id, outcome);
        floating.panels(&mut self.panel_render_trees, module_id, outcome);
        floating.tooltips(&mut self.tooltip_render_trees, outcome);
    }
}
