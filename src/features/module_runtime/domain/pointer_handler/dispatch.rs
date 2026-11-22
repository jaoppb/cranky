use super::action::{PointerAction, PointerOutcome};
use super::handler::PointerHandler;
use super::target::PointerTarget;
use crate::features::module_runtime::domain::Change;
use crate::shared::events::core::PointerEvent;
use crate::shared::primitives::{FunctionName, MonitorId};

impl PointerHandler {
    pub(crate) fn handle_event(
        &mut self,
        event: &PointerEvent,
        target: PointerTarget<'_>,
    ) -> PointerOutcome {
        match event {
            PointerEvent::ButtonPress { pos, .. } => {
                let state_changed = self.handle_button_press(target, *pos);
                PointerOutcome::new(Vec::new(), Change::from(state_changed))
            }
            PointerEvent::ButtonRelease { pos, .. } => {
                let state_changed = self.handle_button_release(target, *pos);
                PointerOutcome::new(Vec::new(), Change::from(state_changed))
            }
            PointerEvent::Click { button, pos, .. } => {
                let actions = Self::handle_click(target, *button, *pos);
                PointerOutcome::new(actions, Change::Unchanged)
            }
            PointerEvent::PointerMotion { pos, .. } => {
                let (actions, state_changed) = self.handle_pointer_motion(target, *pos);
                PointerOutcome::new(actions, Change::from(state_changed))
            }
            PointerEvent::PointerLeave { .. } => {
                let (actions, state_changed) = self.handle_pointer_leave(target.monitor_id());
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
