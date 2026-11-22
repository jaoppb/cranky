use super::runner::EventLoop;
use crate::features::layout_engine::domain::DisplayCommandSender;
use crate::features::module_runtime::domain::pointer_handler::{PointerOutcome, PointerTarget};
use crate::features::module_runtime::domain::{PointerAction, PointerHandler};
use crate::features::module_runtime::ports::{LayoutEventSender, ModuleInitError};
use crate::features::vdom::domain::UiCommandSender;
use crate::shared::events::core::{
    InteractionEvent, PointerEvent, SurfaceKind, SurfaceLifecycleEvent,
};
use crate::shared::primitives::{FunctionName, ModuleId, MonitorId};
use crate::shared::rendering::ports::canvas::CanvasFactory;

impl<
    F: CanvasFactory + 'static,
    LS: LayoutEventSender + 'static,
    DS: DisplayCommandSender + 'static,
    US: UiCommandSender + 'static,
> EventLoop<F, LS, DS, US>
{
    /// Handles one interaction on `monitor_id` and tells whether the module must be rendered
    /// again.
    pub(crate) fn handle_interaction(
        &mut self,
        monitor_id: &MonitorId,
        interaction: InteractionEvent,
    ) -> bool {
        match interaction {
            InteractionEvent::Pointer(pointer_event) => {
                self.handle_pointer_event(monitor_id, &pointer_event)
            }
            InteractionEvent::Lifecycle(lifecycle_event) => {
                self.handle_lifecycle_event(monitor_id, lifecycle_event)
            }
        }
    }

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

        let Some(outcome) = self.hit_test(monitor_id, event) else {
            log_no_render_tree(ctx_id, monitor_id);
            return false;
        };
        self.execute_outcome(&outcome)
    }

    /// Handles `event` against the tree of the surface it happened on, on `monitor_id`; `None`
    /// when that surface has no tree there.
    fn hit_test(&mut self, monitor_id: &MonitorId, event: &PointerEvent) -> Option<PointerOutcome> {
        let render_tree = match event.surface() {
            SurfaceKind::Bar => self.render_pipeline.render_trees().get(monitor_id),
            SurfaceKind::Popup => self
                .popup_render_trees
                .get(monitor_id)
                .map(|(_, node)| node),
            SurfaceKind::Panel => self
                .panel_render_trees
                .get(monitor_id)
                .map(|(_, node)| node),
        }?;
        let target = PointerTarget::new(monitor_id, render_tree);
        Some(self.pointer_handler.handle_event(event, target))
    }

    pub(crate) fn handle_lifecycle_event(
        &mut self,
        monitor_id: &MonitorId,
        event: SurfaceLifecycleEvent,
    ) -> bool {
        tracing::debug!(
            module = %self.ctx.id(),
            monitor = %monitor_id,
            ?event,
            "Received surface lifecycle event in module actor"
        );
        self.forget_dismissed_tree(monitor_id, event);
        let outcome = PointerHandler::handle_dismissal(event, monitor_id);
        self.execute_outcome(&outcome)
    }

    /// Forgets the tree of the floating surface `event` dismissed on `monitor_id`.
    fn forget_dismissed_tree(&mut self, monitor_id: &MonitorId, event: SurfaceLifecycleEvent) {
        match event {
            SurfaceLifecycleEvent::PopupDismissed => self.popup_render_trees.remove(monitor_id),
            SurfaceLifecycleEvent::PanelDismissed => self.panel_render_trees.remove(monitor_id),
        };
    }

    /// Runs `outcome`'s actions in order, refreshes the script once if any call ran, and tells
    /// whether the module must be rendered again.
    fn execute_outcome(&mut self, outcome: &PointerOutcome) -> bool {
        let mut changed = false;
        for action in outcome.actions() {
            match action {
                PointerAction::CallFunction(func_name, mon_id) => {
                    changed |= self.call_script_function(func_name, mon_id.as_ref());
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

    /// Calls the script function `func_name`, with the monitor as its argument when there is one;
    /// true when it ran, false when it failed, which is logged.
    fn call_script_function(
        &mut self,
        func_name: &FunctionName,
        mon_id: Option<&MonitorId>,
    ) -> bool {
        let res = if let Some(monitor) = mon_id {
            self.port
                .call_function_with_args(func_name, &[monitor.as_str()])
        } else {
            self.port.call_function(func_name)
        };
        let Err(error) = res else {
            return true;
        };
        log_script_call_failed(self.ctx.id(), func_name, &error);
        false
    }
}

/// Logs a pointer event module `ctx_id` received on `monitor_id` for a surface it has no tree
/// for.
fn log_no_render_tree(ctx_id: ModuleId, monitor_id: &MonitorId) {
    tracing::warn!(
        module = %ctx_id,
        monitor = %monitor_id,
        "Received pointer event but no render tree found for monitor"
    );
}

/// Logs a script call `func_name` of module `ctx_id` that failed with `error`.
fn log_script_call_failed(ctx_id: ModuleId, func_name: &FunctionName, error: &ModuleInitError) {
    tracing::error!(
        module = %ctx_id,
        func = %func_name,
        "ScriptCall failed: {error}"
    );
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
