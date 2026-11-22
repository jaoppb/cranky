use super::action::PointerAction;
use super::handler::PointerHandler;
use super::hit_ref::capture_node_ref;
use super::target::PointerTarget;
use crate::features::vdom::domain::NodeRef;
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
        let state_changed = self.hover(monitor_id, capture_node_ref(&hit));
        let actions = hit
            .iter()
            .rev()
            .find_map(|node| node.on_hover())
            .map(|cmd| PointerAction::for_ui_action(cmd, monitor_id, pos))
            .into_iter()
            .collect();
        (actions, state_changed)
    }

    /// Makes `target_ref` the node hovered on `monitor_id`, none when it is `None`, and tells
    /// whether that changed it.
    fn hover(&mut self, monitor_id: &MonitorId, target_ref: Option<NodeRef>) -> bool {
        if self.hovered.get(monitor_id) == target_ref.as_ref() {
            return false;
        }
        if let Some(node_ref) = target_ref {
            self.hovered.insert(monitor_id.clone(), node_ref);
        } else {
            self.hovered.remove(monitor_id);
        }
        true
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
