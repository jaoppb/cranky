use super::action::PointerAction;
use super::handler::PointerHandler;
use super::hit_ref::capture_node_ref;
use super::target::PointerTarget;
use crate::shared::events::core::PointerButton;
use crate::shared::primitives::geometry::Position;

impl PointerHandler {
    pub(crate) fn handle_button_press(&mut self, target: PointerTarget<'_>, pos: Position) -> bool {
        let monitor_id = target.monitor_id();
        let hit = target.tree().hit_test(pos);
        let target_ref = capture_node_ref(&hit);
        let old_active = self.active.get(monitor_id).cloned();
        if old_active == target_ref {
            false
        } else {
            if let Some(node_ref) = target_ref {
                self.active.insert(monitor_id.clone(), node_ref);
            } else {
                self.active.remove(monitor_id);
            }
            true
        }
    }

    pub(crate) fn handle_button_release(
        &mut self,
        target: PointerTarget<'_>,
        pos: Position,
    ) -> bool {
        let monitor_id = target.monitor_id();
        let mut changed = self.active.remove(monitor_id).is_some();

        let hit = target.tree().hit_test(pos);
        let target_ref = capture_node_ref(&hit);
        let old_focus = self.focused.get(monitor_id).cloned();
        if old_focus != target_ref {
            if let Some(node_ref) = target_ref {
                self.focused.insert(monitor_id.clone(), node_ref);
            } else {
                self.focused.remove(monitor_id);
            }
            changed = true;
        }

        changed
    }

    pub(crate) fn handle_click(
        target: PointerTarget<'_>,
        button: PointerButton,
        pos: Position,
    ) -> Vec<PointerAction> {
        let monitor_id = target.monitor_id();
        let hit = target.tree().hit_test(pos);
        hit.iter()
            .rev()
            .find_map(|node| node.on_click().and_then(|handlers| handlers.get(button)))
            .map(|cmd| PointerAction::for_ui_action(cmd, monitor_id, pos))
            .into_iter()
            .collect()
    }
}
