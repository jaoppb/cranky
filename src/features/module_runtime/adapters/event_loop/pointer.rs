use super::runner::EventLoop;
use crate::features::layout_engine::domain::DisplayCommandSender;
use crate::features::module_runtime::domain::pointer_handler::{PointerOutcome, PointerTarget};
use crate::features::module_runtime::domain::{PointerAction, PointerHandler};
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
    pub(crate) fn handle_pointer_event(
        &mut self,
        monitor_id: &MonitorId,
        event: &PointerEvent,
    ) -> bool {
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
            crate::shared::events::core::SurfaceKind::Popup => self
                .popup_render_trees
                .get(monitor_id)
                .map(|(_, node)| node),
            crate::shared::events::core::SurfaceKind::Panel => self
                .panel_render_trees
                .get(monitor_id)
                .map(|(_, node)| node),
        };

        if let Some(render_tree) = target_tree {
            let outcome = self
                .pointer_handler
                .handle_event(event, PointerTarget::new(monitor_id, render_tree));
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

    pub(crate) fn handle_lifecycle_event(
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
                self.popup_render_trees.remove(monitor_id);
            }
            crate::shared::events::core::SurfaceLifecycleEvent::PanelDismissed => {
                self.panel_render_trees.remove(monitor_id);
            }
        }
        let outcome = PointerHandler::handle_dismissal(event, monitor_id);
        self.execute_outcome(&outcome)
    }

    fn execute_outcome(&mut self, outcome: &PointerOutcome) -> bool {
        let ctx_id = self.ctx.id();
        let mut changed = false;
        for action in outcome.actions() {
            match action {
                PointerAction::CallFunction(func_name, mon_id) => {
                    let res = if let Some(monitor) = mon_id {
                        self.port
                            .call_function_with_args(func_name, &[monitor.as_str()])
                    } else {
                        self.port.call_function(func_name)
                    };
                    if let Err(error) = res {
                        tracing::error!(
                            module = %ctx_id,
                            func = %func_name,
                            "ScriptCall failed: {error}"
                        );
                    } else {
                        changed = true;
                    }
                }
                PointerAction::SendUi(cmd) => {
                    self.ctx.ui_sender().send_ui_command(cmd.clone());
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

#[cfg(test)]
mod tests {
    use super::super::tests::fixture::{hub_with_dp1, wired_event_loop};
    use crate::features::module_runtime::domain::Change;
    use crate::features::module_runtime::domain::PointerAction;
    use crate::features::module_runtime::domain::pointer_handler::PointerOutcome;
    use crate::features::module_runtime::test_support::RecordingModulePort;
    use crate::features::vdom::domain::{ElementSelectors, Interactions, VNode};
    use crate::shared::primitives::FunctionName;

    /// A call that names no monitor runs the script function with no argument.
    #[test]
    fn a_call_with_no_monitor_runs_the_function_without_arguments() {
        let rect = VNode::new_rect(ElementSelectors::default(), Interactions::default());
        let port = RecordingModulePort::new(rect, vec![]);
        let log = port.log();
        let (mut event_loop, _wiring) = wired_event_loop(port, hub_with_dp1(1.0));
        let reset = PointerAction::CallFunction(FunctionName::new("reset"), None);
        let outcome = PointerOutcome::new(vec![reset], Change::Unchanged);

        assert!(event_loop.execute_outcome(&outcome));
        let calls = std::mem::take(&mut log.lock().unwrap().calls);
        assert_eq!(calls, [("reset".to_string(), Vec::<String>::new())]);
    }
}
