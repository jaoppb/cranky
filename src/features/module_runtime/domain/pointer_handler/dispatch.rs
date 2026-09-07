use super::action::{PointerAction, PointerOutcome};
use super::handler::PointerHandler;
use crate::features::layout_engine::domain::RenderNode;
use crate::shared::events::core::PointerEvent;
use crate::shared::primitives::{FunctionName, MonitorId};

impl PointerHandler {
    pub fn handle_event(
        &mut self,
        event: &PointerEvent,
        monitor_id: &MonitorId,
        render_tree: &RenderNode,
    ) -> PointerOutcome {
        match event {
            PointerEvent::ButtonPress { pos, .. } => {
                let state_changed = self.handle_button_press(monitor_id, *pos, render_tree);
                PointerOutcome::new(Vec::new(), state_changed)
            }
            PointerEvent::ButtonRelease { pos, .. } => {
                let state_changed = self.handle_button_release(monitor_id, *pos, render_tree);
                PointerOutcome::new(Vec::new(), state_changed)
            }
            PointerEvent::Click { button, pos, .. } => {
                let actions = self.handle_click(monitor_id, *button, *pos, render_tree);
                PointerOutcome::new(actions, false)
            }
            PointerEvent::PointerMotion { pos, .. } => {
                let (actions, state_changed) =
                    self.handle_pointer_motion(monitor_id, *pos, render_tree);
                PointerOutcome::new(actions, state_changed)
            }
            PointerEvent::PointerLeave { .. } => {
                let (actions, state_changed) = self.handle_pointer_leave(monitor_id);
                PointerOutcome::new(actions, state_changed)
            }
            _ => PointerOutcome::empty(),
        }
    }

    pub fn handle_dismissal(
        &mut self,
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
                    true,
                )
            }
            crate::shared::events::core::SurfaceLifecycleEvent::PanelDismissed => {
                PointerOutcome::new(
                    vec![PointerAction::CallFunction(
                        FunctionName::new("on_panel_dismiss"),
                        Some(monitor_id.clone()),
                    )],
                    true,
                )
            }
        }
    }
}
