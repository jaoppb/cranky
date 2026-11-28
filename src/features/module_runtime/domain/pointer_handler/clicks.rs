use super::action::PointerAction;
use super::handler::{PointerHandler, replace_node_ref};
use super::hit_ref::capture_node_ref;
use super::target::PointerTarget;
use crate::shared::events::core::PointerButton;
use crate::shared::primitives::geometry::Position;

impl PointerHandler {
    pub(crate) fn handle_button_press(&mut self, target: PointerTarget<'_>, pos: Position) -> bool {
        let monitor_id = target.monitor_id();
        let hit = target.tree().hit_test(pos);
        replace_node_ref(&mut self.active, monitor_id, capture_node_ref(&hit))
    }

    pub(crate) fn handle_button_release(
        &mut self,
        target: PointerTarget<'_>,
        pos: Position,
    ) -> bool {
        let monitor_id = target.monitor_id();
        let ended_press = self.active.remove(monitor_id).is_some();

        let hit = target.tree().hit_test(pos);
        let focus_changed = replace_node_ref(&mut self.focused, monitor_id, capture_node_ref(&hit));
        ended_press || focus_changed
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
