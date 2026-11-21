use super::action::{PointerAction, PointerOutcome};
use super::handler::PointerHandler;
use crate::features::layout_engine::domain::RenderNode;
use crate::features::module_runtime::domain::Change;
use crate::shared::events::core::PointerEvent;
use crate::shared::primitives::{FunctionName, MonitorId};

impl PointerHandler {
    pub(crate) fn handle_event(
        &mut self,
        event: &PointerEvent,
        monitor_id: &MonitorId,
        render_tree: &RenderNode,
    ) -> PointerOutcome {
        match event {
            PointerEvent::ButtonPress { pos, .. } => {
                let state_changed = self.handle_button_press(monitor_id, *pos, render_tree);
                PointerOutcome::new(Vec::new(), Change::from(state_changed))
            }
            PointerEvent::ButtonRelease { pos, .. } => {
                let state_changed = self.handle_button_release(monitor_id, *pos, render_tree);
                PointerOutcome::new(Vec::new(), Change::from(state_changed))
            }
            PointerEvent::Click { button, pos, .. } => {
                let actions = Self::handle_click(monitor_id, *button, *pos, render_tree);
                PointerOutcome::new(actions, Change::Unchanged)
            }
            PointerEvent::PointerMotion { pos, .. } => {
                let (actions, state_changed) =
                    self.handle_pointer_motion(monitor_id, *pos, render_tree);
                PointerOutcome::new(actions, Change::from(state_changed))
            }
            PointerEvent::PointerLeave { .. } => {
                let (actions, state_changed) = self.handle_pointer_leave(monitor_id);
                PointerOutcome::new(actions, Change::from(state_changed))
            }
            _ => PointerOutcome::empty(),
        }
    }

    pub(crate) fn handle_dismissal(
        event: crate::shared::events::core::SurfaceLifecycleEvent,
        monitor_id: &MonitorId,
    ) -> PointerOutcome {
        match event {
            crate::shared::events::core::SurfaceLifecycleEvent::PopupDismissed => {
                PointerOutcome::new(
                    vec![PointerAction::CallFunction(
                        FunctionName::new("on_popup_dismiss"),
                        Some(monitor_id.clone()),
                    )],
                    Change::Changed,
                )
            }
            crate::shared::events::core::SurfaceLifecycleEvent::PanelDismissed => {
                PointerOutcome::new(
                    vec![PointerAction::CallFunction(
                        FunctionName::new("on_panel_dismiss"),
                        Some(monitor_id.clone()),
                    )],
                    Change::Changed,
                )
            }
        }
    }
}
