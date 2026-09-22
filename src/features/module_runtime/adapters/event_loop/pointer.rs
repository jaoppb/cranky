use super::runner::EventLoop;
use crate::features::layout_engine::domain::DisplayCommandSender;
use crate::features::module_runtime::domain::PointerAction;
use crate::features::module_runtime::domain::pointer_handler::PointerOutcome;
use crate::features::module_runtime::ports::LayoutEventSender;
use crate::features::vdom::domain::UiCommandSender;
use crate::shared::events::core::PointerEvent;
use crate::shared::primitives::MonitorId;
use crate::shared::rendering::ports::canvas::CanvasFactory;

impl<
    F: CanvasFactory + 'static,
    LS: LayoutEventSender + 'static,
    DS: DisplayCommandSender + 'static,
    US: UiCommandSender + 'static,
> EventLoop<F, LS, DS, US>
{
    pub fn handle_pointer_event(&mut self, monitor_id: &MonitorId, event: &PointerEvent) -> bool {
        let ctx_id = self.ctx.id();
        tracing::debug!(
            module = %ctx_id,
            monitor = %monitor_id,
            event = ?event,
            "Received pointer event in module actor"
        );

        let target_tree = match event.surface() {
            crate::shared::events::core::SurfaceKind::Bar => {
                self.render_pipeline.render_trees().get(monitor_id)
            }
            crate::shared::events::core::SurfaceKind::Popup => {
                self.floating.popup_render_trees().get(monitor_id)
            }
            crate::shared::events::core::SurfaceKind::Panel => {
                self.floating.panel_render_trees().get(monitor_id)
            }
        };

        if let Some(render_tree) = target_tree {
            let outcome = self
                .pointer_handler
                .handle_event(event, monitor_id, render_tree);
            self.execute_outcome(&outcome)
        } else {
            tracing::warn!(
                module = %ctx_id,
                monitor = %monitor_id,
                "Received pointer event but no render tree found for monitor"
            );
            false
        }
    }

    pub fn handle_lifecycle_event(
        &mut self,
        monitor_id: &MonitorId,
        event: crate::shared::events::core::SurfaceLifecycleEvent,
    ) -> bool {
        let ctx_id = self.ctx.id();
        tracing::debug!(
            module = %ctx_id,
            monitor = %monitor_id,
            ?event,
            "Received surface lifecycle event in module actor"
        );
        match event {
            crate::shared::events::core::SurfaceLifecycleEvent::PopupDismissed => {
                self.floating.popup_render_trees_mut().remove(monitor_id);
            }
            crate::shared::events::core::SurfaceLifecycleEvent::PanelDismissed => {
                self.floating.panel_render_trees_mut().remove(monitor_id);
            }
        }
        let outcome = self.pointer_handler.handle_dismissal(event, monitor_id);
        self.execute_outcome(&outcome)
    }

    fn execute_outcome(&mut self, outcome: &PointerOutcome) -> bool {
        let ctx_id = self.ctx.id();
        let mut changed = false;
        for action in outcome.actions() {
            match action {
                PointerAction::CallFunction(func_name, mon_id) => {
                    let res = if let Some(m) = mon_id {
                        self.port.call_function_with_args(func_name, &[m.as_str()])
                    } else {
                        self.port.call_function(func_name)
                    };
                    if let Err(e) = res {
                        tracing::error!(
                            module = %ctx_id,
                            func = %func_name,
                            "ScriptCall failed: {e}"
                        );
                    } else {
                        changed = true;
                    }
                }
                PointerAction::SendUi(cmd) => {
                    self.ctx.ui_sender().send_ui_command(cmd.clone());
                }
                PointerAction::SendDisplay(cmd) => {
                    self.ctx.display_sender().send_display_command(cmd.clone());
                }
            }
        }
        if changed {
            let subs = self.port.subscriptions().to_vec();
            self.port.refresh(self.ctx.hub(), &subs);
        }
        changed || outcome.has_state_changed()
    }
}
