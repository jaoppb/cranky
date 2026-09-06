use super::action::PointerAction;
use super::handler::PointerHandler;
use crate::features::layout_engine::domain::RenderNode;
use crate::features::vdom::domain::{UiAction, UiCommand};
use crate::shared::events::core::PointerButton;
use crate::shared::primitives::geometry::Position;
use crate::shared::primitives::MonitorId;

impl PointerHandler {
    pub fn handle_button_press(
        &mut self,
        monitor_id: &MonitorId,
        pos: Position,
        render_tree: &RenderNode,
    ) -> bool {
        self.last_pointer_pos = Some((monitor_id.clone(), pos));
        let hit = render_tree.hit_test(pos);
        let target_path = hit.last().map(|n| n.path().clone());
        let old_active = self.active_nodes.get(monitor_id).cloned();
        if old_active == target_path {
            false
        } else {
            if let Some(p) = target_path {
                self.active_nodes.insert(monitor_id.clone(), p);
            } else {
                self.active_nodes.remove(monitor_id);
            }
            true
        }
    }

    pub fn handle_button_release(
        &mut self,
        monitor_id: &MonitorId,
        pos: Position,
        render_tree: &RenderNode,
    ) -> bool {
        self.last_pointer_pos = Some((monitor_id.clone(), pos));
        let mut changed = self.active_nodes.remove(monitor_id).is_some();

        let hit = render_tree.hit_test(pos);
        let target_path = hit.last().map(|n| n.path().clone());
        let old_focus = self.focused_nodes.get(monitor_id).cloned();
        if old_focus != target_path {
            if let Some(p) = target_path {
                self.focused_nodes.insert(monitor_id.clone(), p);
            } else {
                self.focused_nodes.remove(monitor_id);
            }
            changed = true;
        }

        changed
    }

    pub(crate) fn handle_click(
        &mut self,
        monitor_id: &MonitorId,
        button: PointerButton,
        pos: Position,
        render_tree: &RenderNode,
    ) -> Vec<PointerAction> {
        self.last_pointer_pos = Some((monitor_id.clone(), pos));
        let hit = render_tree.hit_test(pos);
        let hit_cmd = hit
            .iter()
            .rev()
            .find_map(|n| n.on_click().and_then(|h| h.get(&button)));

        let mut actions = Vec::new();
        if let Some(cmd) = hit_cmd {
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
        actions
    }
}
