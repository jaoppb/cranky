use super::action::PointerAction;
use super::handler::PointerHandler;
use crate::features::layout_engine::domain::{DisplayCommand, FloatingKind, RenderNode};
use crate::features::vdom::domain::{NodeRef, UiAction, UiCommand};
use crate::shared::primitives::geometry::Position;
use crate::shared::primitives::MonitorId;
use std::collections::HashMap;

impl PointerHandler {
    pub(crate) fn handle_pointer_motion(
        &mut self,
        monitor_id: &MonitorId,
        pos: Position,
        render_tree: &RenderNode,
    ) -> (Vec<PointerAction>, bool) {
        let mut actions = Vec::new();
        let mut state_changed = false;

        self.last_pointer_pos = Some((monitor_id.clone(), pos));
        let hit = render_tree.hit_test(pos);
        let target_ref = hit
            .last()
            .map(|n| NodeRef::new(n.path().clone(), n.node_key().cloned()));
        let old_hover = self.hovered_nodes.get(monitor_id).cloned();
        if old_hover != target_ref {
            if let Some(r) = &target_ref {
                self.hovered_nodes.insert(monitor_id.clone(), r.clone());
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

        let hit_tooltip = hit.iter().rev().find_map(|n| n.tooltip()).cloned();
        if hit_tooltip != self.last_tooltip {
            if let Some(layout) = &hit_tooltip {
                actions.push(PointerAction::SendDisplay(
                    DisplayCommand::ShowFloatingSurface {
                        kind: FloatingKind::Tooltip,
                        monitor_id: Some(monitor_id.clone()),
                        anchor_rect: None,
                        layout: Box::new(layout.clone()),
                        offset: None,
                    },
                ));
            } else {
                actions.push(PointerAction::SendDisplay(
                    DisplayCommand::HideFloatingSurface {
                        kind: FloatingKind::Tooltip,
                    },
                ));
            }
            self.last_tooltip = hit_tooltip;
        }

        (actions, state_changed)
    }

    pub(crate) fn handle_pointer_leave(
        &mut self,
        monitor_id: &MonitorId,
    ) -> (Vec<PointerAction>, bool) {
        self.last_pointer_pos = None;
        let had_hover = self.hovered_nodes.remove(monitor_id).is_some();
        let had_active = self.active_nodes.remove(monitor_id).is_some();
        let state_changed = had_hover || had_active;

        let mut actions = Vec::new();
        if self.last_tooltip.is_some() {
            actions.push(PointerAction::SendDisplay(
                DisplayCommand::HideFloatingSurface {
                    kind: FloatingKind::Tooltip,
                },
            ));
            self.last_tooltip = None;
        }

        (actions, state_changed)
    }

    pub fn update_after_render(
        &mut self,
        render_trees: &HashMap<MonitorId, RenderNode>,
    ) -> Vec<PointerAction> {
        let mut actions = Vec::new();

        if let Some((monitor_id, pos)) = &self.last_pointer_pos
            && let Some(render_tree) = render_trees.get(monitor_id)
        {
            let hit = render_tree.hit_test(*pos);
            let hit_tooltip = hit.iter().rev().find_map(|n| n.tooltip()).cloned();
            if hit_tooltip != self.last_tooltip {
                if let Some(layout) = &hit_tooltip {
                    actions.push(PointerAction::SendDisplay(
                        DisplayCommand::ShowFloatingSurface {
                            kind: FloatingKind::Tooltip,
                            monitor_id: Some(monitor_id.clone()),
                            anchor_rect: None,
                            layout: Box::new(layout.clone()),
                            offset: None,
                        },
                    ));
                } else {
                    actions.push(PointerAction::SendDisplay(
                        DisplayCommand::HideFloatingSurface {
                            kind: FloatingKind::Tooltip,
                        },
                    ));
                }
                self.last_tooltip = hit_tooltip;
            }
        }

        actions
    }
}
