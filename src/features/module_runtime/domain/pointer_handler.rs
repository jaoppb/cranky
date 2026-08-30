use crate::app::commands::AppCommand;
use crate::features::layout_engine::domain::{NodePath, RenderNode, StyledNode};
use crate::features::vdom::domain::InteractionContext;
use crate::shared::events::core::{PointerButton, PointerEvent};
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

    fn handle_button_press(
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
            if let Some(p) = &target_path {
                self.active_nodes.insert(monitor_id.clone(), p.clone());
            } else {
                self.active_nodes.remove(monitor_id);
            }
            true
        }
    }

    fn handle_button_release(
        &mut self,
        monitor_id: &MonitorId,
        pos: Position,
        render_tree: &RenderNode,
    ) -> bool {
        let had_active = self.active_nodes.remove(monitor_id).is_some();
        let hit = render_tree.hit_test(pos);
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
        had_active || focus_changed
    }

    fn handle_click(
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
                AppCommand::ScriptCall(func_name) => {
                    actions.push(PointerAction::CallFunction(func_name.clone()));
                }
                AppCommand::SystrayAction { id, action, .. } => {
                    actions.push(PointerAction::SendCommand(AppCommand::SystrayAction {
                        id: id.clone(),
                        action: action.clone(),
                        pos: Some(pos),
                    }));
                }
                other => {
                    actions.push(PointerAction::SendCommand(other.clone()));
                }
            }
        }
        actions
    }

    fn handle_pointer_motion(
        &mut self,
        monitor_id: &MonitorId,
        pos: Position,
        render_tree: &RenderNode,
    ) -> (Vec<PointerAction>, bool) {
        let mut actions = Vec::new();
        let mut state_changed = false;

        self.last_pointer_pos = Some((monitor_id.clone(), pos));
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

        (actions, state_changed)
    }

    fn handle_pointer_leave(&mut self, monitor_id: &MonitorId) -> (Vec<PointerAction>, bool) {
        self.last_pointer_pos = None;
        let had_hover = self.hovered_nodes.remove(monitor_id).is_some();
        let had_active = self.active_nodes.remove(monitor_id).is_some();
        let state_changed = had_hover || had_active;

        let mut actions = Vec::new();
        if self.last_tooltip.is_some() {
            actions.push(PointerAction::SendCommand(AppCommand::HideTooltip));
            self.last_tooltip = None;
        }

        (actions, state_changed)
    }

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
            PointerEvent::Click { button, pos } => {
                let actions = self.handle_click(monitor_id, *button, *pos, render_tree);
                PointerOutcome::new(actions, false)
            }
            PointerEvent::PointerMotion { pos } => {
                let (actions, state_changed) =
                    self.handle_pointer_motion(monitor_id, *pos, render_tree);
                PointerOutcome::new(actions, state_changed)
            }
            PointerEvent::PointerLeave => {
                let (actions, state_changed) = self.handle_pointer_leave(monitor_id);
                PointerOutcome::new(actions, state_changed)
            }
            _ => PointerOutcome::empty(),
        }
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

    use crate::features::vdom::domain::ClickHandlers;

    fn make_test_tree(
        on_click: Option<ClickHandlers>,
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
    fn test_click_hit_returns_send_command_single_shorthand() {
        let mut handler = PointerHandler::new();
        let mon = MonitorId::new("DP-1");
        let tree = make_test_tree(
            Some(ClickHandlers::from_single(AppCommand::RequestRender)),
            None,
            None,
        );

        // Left click
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

        // Right click also triggers on single action shorthand
        let outcome_right = handler.handle_event(
            &PointerEvent::Click {
                button: PointerButton::Right,
                pos: Position::new(10, 10),
            },
            &mon,
            &tree,
        );
        assert_eq!(outcome_right.actions().len(), 1);
        assert_eq!(
            outcome_right.actions()[0],
            PointerAction::SendCommand(AppCommand::RequestRender)
        );

        // Middle click also triggers
        let outcome_middle = handler.handle_event(
            &PointerEvent::Click {
                button: PointerButton::Middle,
                pos: Position::new(10, 10),
            },
            &mon,
            &tree,
        );
        assert_eq!(outcome_middle.actions().len(), 1);
        assert_eq!(
            outcome_middle.actions()[0],
            PointerAction::SendCommand(AppCommand::RequestRender)
        );

        // Auxiliary button does not trigger single shorthand (unless mapped)
        let outcome_side = handler.handle_event(
            &PointerEvent::Click {
                button: PointerButton::Side,
                pos: Position::new(10, 10),
            },
            &mon,
            &tree,
        );
        assert!(outcome_side.actions().is_empty());
    }

    #[test]
    fn test_click_hit_returns_distinct_button_actions() {
        let mut handler = PointerHandler::new();
        let mon = MonitorId::new("DP-1");
        let mut handlers = ClickHandlers::new();
        handlers.insert(PointerButton::Left, AppCommand::Exec("left_clicked".into()));
        handlers.insert(
            PointerButton::Right,
            AppCommand::Exec("right_clicked".into()),
        );
        handlers.insert(PointerButton::Side, AppCommand::Exec("side_clicked".into()));

        let tree = make_test_tree(Some(handlers), None, None);

        let outcome_left = handler.handle_event(
            &PointerEvent::Click {
                button: PointerButton::Left,
                pos: Position::new(10, 10),
            },
            &mon,
            &tree,
        );
        assert_eq!(
            outcome_left.into_actions(),
            vec![PointerAction::SendCommand(AppCommand::Exec(
                "left_clicked".into()
            ))]
        );

        let outcome_right = handler.handle_event(
            &PointerEvent::Click {
                button: PointerButton::Right,
                pos: Position::new(10, 10),
            },
            &mon,
            &tree,
        );
        assert_eq!(
            outcome_right.into_actions(),
            vec![PointerAction::SendCommand(AppCommand::Exec(
                "right_clicked".into()
            ))]
        );

        let outcome_side = handler.handle_event(
            &PointerEvent::Click {
                button: PointerButton::Side,
                pos: Position::new(10, 10),
            },
            &mon,
            &tree,
        );
        assert_eq!(
            outcome_side.into_actions(),
            vec![PointerAction::SendCommand(AppCommand::Exec(
                "side_clicked".into()
            ))]
        );

        let outcome_middle = handler.handle_event(
            &PointerEvent::Click {
                button: PointerButton::Middle,
                pos: Position::new(10, 10),
            },
            &mon,
            &tree,
        );
        assert!(outcome_middle.actions().is_empty());
    }

    #[test]
    fn test_click_script_call_returns_call_function() {
        let mut handler = PointerHandler::new();
        let mon = MonitorId::new("DP-1");
        let func_name = FunctionName::new("toggle_menu");
        let tree = make_test_tree(
            Some(ClickHandlers::from_single(AppCommand::ScriptCall(
                func_name.clone(),
            ))),
            None,
            None,
        );

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
    fn test_click_systray_action_forwards_position() {
        let mut handler = PointerHandler::new();
        let mon = MonitorId::new("DP-1");
        let systray_cmd = AppCommand::SystrayAction {
            id: crate::features::systray::domain::SystrayId::new("test_item"),
            action: crate::features::systray::domain::SystrayActionName::ContextMenu,
            pos: None,
        };
        let tree = make_test_tree(Some(ClickHandlers::from_single(systray_cmd)), None, None);

        let outcome = handler.handle_event(
            &PointerEvent::Click {
                button: PointerButton::Right,
                pos: Position::new(42, 84),
            },
            &mon,
            &tree,
        );

        assert_eq!(
            outcome.into_actions(),
            vec![PointerAction::SendCommand(AppCommand::SystrayAction {
                id: crate::features::systray::domain::SystrayId::new("test_item"),
                action: crate::features::systray::domain::SystrayActionName::ContextMenu,
                pos: Some(Position::new(42, 84)),
            })]
        );
    }

    #[test]
    fn test_pointer_outcome_methods() {
        let empty = PointerOutcome::empty();
        assert!(empty.actions().is_empty());
        assert!(!empty.has_state_changed());
        assert_eq!(empty.into_actions(), Vec::<PointerAction>::new());
    }
}
