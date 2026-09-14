use super::action::PointerAction;
use super::handler::PointerHandler;
use crate::features::layout_engine::domain::RenderNode;
use crate::features::vdom::domain::{UiAction, UiCommand};
use crate::shared::primitives::geometry::Position;
use crate::shared::primitives::MonitorId;

impl PointerHandler {
    pub(crate) fn handle_pointer_motion(
        &mut self,
        monitor_id: &MonitorId,
        pos: Position,
        render_tree: &RenderNode,
    ) -> (Vec<PointerAction>, bool) {
        let mut actions = Vec::new();
        let mut state_changed = false;

        let hit = render_tree.hit_test(pos);
        let target_path = hit.last().map(|n| n.path().clone());
        let old_hover = self.hovered_nodes.get(monitor_id).cloned();
        if old_hover != target_path {
            if let Some(p) = &target_path {
                self.hovered_nodes.insert(monitor_id.clone(), p.clone());
            } else {
                self.hovered_nodes.remove(monitor_id);
            }
            state_changed = true;
        }

        if let Some(cmd) = hit.iter().rev().find_map(|n| n.on_hover()) {
            match cmd {
                UiAction::ScriptCall(func_name) => {
                    actions.push(PointerAction::CallFunction(
                        func_name.clone(),
                        Some(monitor_id.clone()),
                    ));
                }
                UiAction::SystrayAction { id, action, .. } => {
                    actions.push(PointerAction::SendUi(UiCommand::SystrayAction {
                        id: id.clone(),
                        action: action.clone(),
                        pos: Some(pos),
                    }));
                }
                UiAction::Exec(cmd_str) => {
                    actions.push(PointerAction::SendUi(UiCommand::Exec(cmd_str.clone())));
                }
            }
        }

        (actions, state_changed)
    }

    pub(crate) fn handle_pointer_leave(
        &mut self,
        monitor_id: &MonitorId,
    ) -> (Vec<PointerAction>, bool) {
        let had_hover = self.hovered_nodes.remove(monitor_id).is_some();
        let had_active = self.active_nodes.remove(monitor_id).is_some();
        let state_changed = had_hover || had_active;

        (Vec::new(), state_changed)
    }
}
