use crate::app::commands::AppCommand;
use crate::features::layout_engine::domain::{NodePath, RenderNode, StyledNode};
use crate::features::vdom::domain::InteractionContext;
use crate::shared::events::core::PointerEvent;
use crate::shared::primitives::geometry::Position;
use crate::shared::primitives::{FunctionName, MonitorId};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum PointerAction {
    SendCommand(AppCommand),
    CallFunction(FunctionName),
}

#[derive(Debug, Clone, PartialEq)]
pub struct PointerOutcome {
    actions: Vec<PointerAction>,
    state_changed: bool,
}

impl PointerOutcome {
    #[must_use]
    pub const fn new(actions: Vec<PointerAction>, state_changed: bool) -> Self {
        Self {
            actions,
            state_changed,
        }
    }

    #[must_use]
    pub const fn empty() -> Self {
        Self {
            actions: Vec::new(),
            state_changed: false,
        }
    }

    #[must_use]
    pub fn actions(&self) -> &[PointerAction] {
        &self.actions
    }

    #[must_use]
    pub const fn has_state_changed(&self) -> bool {
        self.state_changed
    }

    #[must_use]
    pub fn into_actions(self) -> Vec<PointerAction> {
        self.actions
    }
}

#[derive(Debug, Default)]
pub struct PointerHandler {
    hovered_nodes: HashMap<MonitorId, NodePath>,
    active_nodes: HashMap<MonitorId, NodePath>,
    focused_nodes: HashMap<MonitorId, NodePath>,
    last_tooltip: Option<StyledNode>,
    last_pointer_pos: Option<(MonitorId, Position)>,
}

impl PointerHandler {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn hovered_node(&self, monitor_id: &MonitorId) -> Option<&NodePath> {
        self.hovered_nodes.get(monitor_id)
    }

    #[must_use]
    pub fn active_node(&self, monitor_id: &MonitorId) -> Option<&NodePath> {
        self.active_nodes.get(monitor_id)
    }

    #[must_use]
    pub fn focused_node(&self, monitor_id: &MonitorId) -> Option<&NodePath> {
        self.focused_nodes.get(monitor_id)
    }

    #[must_use]
    pub fn interaction_context(
        &self,
        monitor_id: &MonitorId,
        is_monitor_focused: bool,
    ) -> InteractionContext {
        InteractionContext::new(
            self.hovered_nodes.get(monitor_id).cloned(),
            self.active_nodes.get(monitor_id).cloned(),
            self.focused_nodes.get(monitor_id).cloned(),
            is_monitor_focused,
        )
    }

    #[must_use]
    pub const fn last_tooltip(&self) -> Option<&StyledNode> {
        self.last_tooltip.as_ref()
    }

    #[must_use]
    pub const fn last_pointer_pos(&self) -> Option<&(MonitorId, Position)> {
        self.last_pointer_pos.as_ref()
    }

    pub fn handle_event(
        &mut self,
        event: &PointerEvent,
        monitor_id: &MonitorId,
        render_tree: &RenderNode,
    ) -> PointerOutcome {
        let mut actions = Vec::new();
        let mut state_changed = false;

        match event {
            PointerEvent::ButtonPress { pos, .. } => {
                self.last_pointer_pos = Some((monitor_id.clone(), *pos));
                let hit = render_tree.hit_test(*pos);
                let target_path = hit.last().map(|n| n.path().clone());
                let old_active = self.active_nodes.get(monitor_id).cloned();
                if old_active != target_path {
                    if let Some(p) = &target_path {
                        self.active_nodes.insert(monitor_id.clone(), p.clone());
                    } else {
                        self.active_nodes.remove(monitor_id);
                    }
                    state_changed = true;
                }
            }
            PointerEvent::ButtonRelease { pos, .. } => {
                let had_active = self.active_nodes.remove(monitor_id).is_some();
                let hit = render_tree.hit_test(*pos);
                let target_path = hit.last().map(|n| n.path().clone());
                let old_focused = self.focused_nodes.get(monitor_id).cloned();
                let focus_changed = old_focused != target_path;
                if focus_changed {
                    if let Some(p) = &target_path {
                        self.focused_nodes.insert(monitor_id.clone(), p.clone());
                    } else {
                        self.focused_nodes.remove(monitor_id);
                    }
                }
                if had_active || focus_changed {
                    state_changed = true;
                }
            }
            PointerEvent::Click { pos, .. } => {
                self.last_pointer_pos = Some((monitor_id.clone(), *pos));
                let hit = render_tree.hit_test(*pos);
                let hit_cmd = hit.iter().rev().find_map(|n| n.on_click());

                if let Some(cmd) = hit_cmd {
                    if let AppCommand::ScriptCall(func_name) = cmd {
                        actions.push(PointerAction::CallFunction(func_name.clone()));
                    } else {
                        actions.push(PointerAction::SendCommand(cmd.clone()));
                    }
                }
            }
            PointerEvent::PointerMotion { pos } => {
                self.last_pointer_pos = Some((monitor_id.clone(), *pos));
                let hit = render_tree.hit_test(*pos);
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

                let hit_cmd = hit.iter().rev().find_map(|n| n.on_hover());
                if let Some(cmd) = hit_cmd {
                    actions.push(PointerAction::SendCommand(cmd.clone()));
                }

                let hit_tooltip = hit.iter().rev().find_map(|n| n.tooltip()).cloned();
                if hit_tooltip != self.last_tooltip {
                    if let Some(layout) = &hit_tooltip {
                        actions.push(PointerAction::SendCommand(AppCommand::ShowTooltip {
                            layout: Box::new(layout.clone()),
                        }));
                    } else {
                        actions.push(PointerAction::SendCommand(AppCommand::HideTooltip));
                    }
                    self.last_tooltip = hit_tooltip;
                }
            }
            PointerEvent::PointerLeave => {
                self.last_pointer_pos = None;
                let had_hover = self.hovered_nodes.remove(monitor_id).is_some();
                let had_active = self.active_nodes.remove(monitor_id).is_some();
                if had_hover || had_active {
                    state_changed = true;
                }
                if self.last_tooltip.is_some() {
                    actions.push(PointerAction::SendCommand(AppCommand::HideTooltip));
                    self.last_tooltip = None;
                }
            }
            _ => {}
        }

        PointerOutcome::new(actions, state_changed)
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
                    actions.push(PointerAction::SendCommand(AppCommand::ShowTooltip {
                        layout: Box::new(layout.clone()),
                    }));
                } else {
                    actions.push(PointerAction::SendCommand(AppCommand::HideTooltip));
                }
                self.last_tooltip = hit_tooltip;
            }
        }

        actions
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::layout_engine::domain::RenderNode;
    use crate::features::styling::domain::ComputedStyle;
    use crate::shared::events::core::PointerButton;
    use crate::shared::primitives::geometry::{Rect, Size};

    fn make_test_tree(
        on_click: Option<AppCommand>,
        on_hover: Option<AppCommand>,
        tooltip: Option<StyledNode>,
    ) -> RenderNode {
        RenderNode::Rect {
            path: NodePath::root(),
            rect: Rect::new(Position::new(0, 0), Size::new(100, 100)),
            style: ComputedStyle::default(),
            on_click,
            on_hover,
            tooltip: tooltip.map(Box::new),
        }
    }

    #[test]
    fn test_click_hit_returns_send_command() {
        let mut handler = PointerHandler::new();
        let mon = MonitorId::new("DP-1");
        let tree = make_test_tree(Some(AppCommand::RequestRender), None, None);

        let outcome = handler.handle_event(
            &PointerEvent::Click {
                button: PointerButton::Left,
                pos: Position::new(10, 10),
            },
            &mon,
            &tree,
        );

        assert_eq!(outcome.actions().len(), 1);
        assert_eq!(
            outcome.actions()[0],
            PointerAction::SendCommand(AppCommand::RequestRender)
        );
        assert!(!outcome.has_state_changed());
    }

    #[test]
    fn test_click_script_call_returns_call_function() {
        let mut handler = PointerHandler::new();
        let mon = MonitorId::new("DP-1");
        let func_name = FunctionName::new("toggle_menu");
        let tree = make_test_tree(Some(AppCommand::ScriptCall(func_name.clone())), None, None);

        let outcome = handler.handle_event(
            &PointerEvent::Click {
                button: PointerButton::Left,
                pos: Position::new(10, 10),
            },
            &mon,
            &tree,
        );

        assert_eq!(outcome.actions().len(), 1);
        assert_eq!(
            outcome.into_actions(),
            vec![PointerAction::CallFunction(func_name)]
        );
    }

    #[test]
    fn test_interaction_state_lifecycle() {
        let mut handler = PointerHandler::new();
        let mon = MonitorId::new("DP-1");
        let tree = make_test_tree(None, None, None);

        // 1. Motion -> Hover
        let outcome_motion = handler.handle_event(
            &PointerEvent::PointerMotion {
                pos: Position::new(10, 10),
            },
            &mon,
            &tree,
        );
        assert!(outcome_motion.has_state_changed());
        assert_eq!(handler.hovered_node(&mon), Some(&NodePath::root()));

        // 2. ButtonPress -> Active
        let outcome_press = handler.handle_event(
            &PointerEvent::ButtonPress {
                button: PointerButton::Left,
                pos: Position::new(10, 10),
            },
            &mon,
            &tree,
        );
        assert!(outcome_press.has_state_changed());
        assert_eq!(handler.active_node(&mon), Some(&NodePath::root()));

        // 3. ButtonRelease -> Clear Active, Set Focus
        let outcome_release = handler.handle_event(
            &PointerEvent::ButtonRelease {
                button: PointerButton::Left,
                pos: Position::new(10, 10),
            },
            &mon,
            &tree,
        );
        assert!(outcome_release.has_state_changed());
        assert_eq!(handler.active_node(&mon), None);
        assert_eq!(handler.focused_node(&mon), Some(&NodePath::root()));

        // 4. PointerLeave -> Clear Hover
        let outcome_leave = handler.handle_event(&PointerEvent::PointerLeave, &mon, &tree);
        assert!(outcome_leave.has_state_changed());
        assert_eq!(handler.hovered_node(&mon), None);
        // Focus is preserved across leave
        assert_eq!(handler.focused_node(&mon), Some(&NodePath::root()));
    }

    #[test]
    fn test_motion_with_tooltip_lifecycle() {
        let mut handler = PointerHandler::new();
        let mon = MonitorId::new("DP-1");
        let tooltip_node = StyledNode::Text {
            path: NodePath::root(),
            text: crate::features::vdom::domain::TextContent::new("hello".to_string()),
            style: ComputedStyle::default(),
            on_click: None,
            on_hover: None,
            tooltip: None,
        };
        let tree = make_test_tree(None, None, Some(tooltip_node.clone()));

        // Motion inside -> ShowTooltip
        let outcome = handler.handle_event(
            &PointerEvent::PointerMotion {
                pos: Position::new(10, 10),
            },
            &mon,
            &tree,
        );
        assert_eq!(outcome.actions().len(), 1);
        match &outcome.actions()[0] {
            PointerAction::SendCommand(AppCommand::ShowTooltip { layout }) => {
                assert_eq!(**layout, tooltip_node);
            }
            _ => panic!("Expected ShowTooltip"),
        }

        // Motion again on same tooltip -> no new ShowTooltip action
        let outcome2 = handler.handle_event(
            &PointerEvent::PointerMotion {
                pos: Position::new(12, 12),
            },
            &mon,
            &tree,
        );
        assert!(outcome2.actions().is_empty());

        // PointerLeave -> HideTooltip
        let outcome3 = handler.handle_event(&PointerEvent::PointerLeave, &mon, &tree);
        assert_eq!(outcome3.actions().len(), 1);
        assert_eq!(
            outcome3.actions()[0],
            PointerAction::SendCommand(AppCommand::HideTooltip)
        );
        assert!(handler.last_tooltip().is_none());
    }

    #[test]
    fn test_update_after_render_detects_tooltip_change() {
        let mut handler = PointerHandler::new();
        let mon = MonitorId::new("DP-1");
        let tree1 = make_test_tree(None, None, None);

        // Move to (10, 10)
        let _ = handler.handle_event(
            &PointerEvent::PointerMotion {
                pos: Position::new(10, 10),
            },
            &mon,
            &tree1,
        );

        // Render update adds tooltip under pointer
        let tooltip_node = StyledNode::Text {
            path: NodePath::root(),
            text: crate::features::vdom::domain::TextContent::new("dynamic".to_string()),
            style: ComputedStyle::default(),
            on_click: None,
            on_hover: None,
            tooltip: None,
        };
        let tree2 = make_test_tree(None, None, Some(tooltip_node.clone()));
        let mut render_trees = HashMap::new();
        render_trees.insert(mon.clone(), tree2);

        let actions = handler.update_after_render(&render_trees);
        assert_eq!(actions.len(), 1);
        match &actions[0] {
            PointerAction::SendCommand(AppCommand::ShowTooltip { layout }) => {
                assert_eq!(**layout, tooltip_node);
            }
            _ => panic!("Expected ShowTooltip"),
        }
    }

    #[test]
    fn test_pointer_outcome_methods() {
        let empty = PointerOutcome::empty();
        assert!(empty.actions().is_empty());
        assert!(!empty.has_state_changed());
        assert_eq!(empty.into_actions(), Vec::<PointerAction>::new());
    }
}
