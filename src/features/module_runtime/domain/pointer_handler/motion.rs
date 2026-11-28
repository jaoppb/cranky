use super::action::PointerAction;
use super::handler::{PointerHandler, replace_node_ref};
use super::hit_ref::capture_node_ref;
use super::target::PointerTarget;
use crate::shared::primitives::MonitorId;
use crate::shared::primitives::geometry::Position;

impl PointerHandler {
    pub(crate) fn handle_pointer_motion(
        &mut self,
        target: PointerTarget<'_>,
        pos: Position,
    ) -> (Vec<PointerAction>, bool) {
        let monitor_id = target.monitor_id();
        let hit = target.tree().hit_test(pos);
        let state_changed = replace_node_ref(&mut self.hovered, monitor_id, capture_node_ref(&hit));
        let actions = hit
            .iter()
            .rev()
            .find_map(|node| node.on_hover())
            .map(|cmd| PointerAction::for_ui_action(cmd, monitor_id, pos))
            .into_iter()
            .collect();
        (actions, state_changed)
    }

    pub(crate) fn handle_pointer_leave(
        &mut self,
        monitor_id: &MonitorId,
    ) -> (Vec<PointerAction>, bool) {
        let had_hover = self.hovered.remove(monitor_id).is_some();
        let had_active = self.active.remove(monitor_id).is_some();
        let state_changed = had_hover || had_active;

        (Vec::new(), state_changed)
    }
}
