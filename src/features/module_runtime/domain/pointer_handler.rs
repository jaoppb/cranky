use crate::app::commands::AppCommand;
use crate::features::layout_engine::domain::{RenderNode, StyledNode};
use crate::shared::events::core::PointerEvent;
use crate::shared::primitives::geometry::Position;
use crate::shared::primitives::{FunctionName, MonitorId};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum PointerAction {
    SendCommand(AppCommand),
    CallFunction(FunctionName),
}

#[derive(Debug, Default)]
pub struct PointerHandler {
    last_tooltip: Option<StyledNode>,
    last_pointer_pos: Option<(MonitorId, Position)>,
}

impl PointerHandler {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn last_tooltip(&self) -> Option<&StyledNode> {
        self.last_tooltip.as_ref()
    }

    pub fn last_pointer_pos(&self) -> Option<&(MonitorId, Position)> {
        self.last_pointer_pos.as_ref()
    }

    pub fn handle_event(
        &mut self,
        event: &PointerEvent,
        monitor_id: &MonitorId,
        render_tree: &RenderNode,
    ) -> Vec<PointerAction> {
        let mut actions = Vec::new();

        match event {
            PointerEvent::Click { x, y, .. } => {
                let pos = Position::new(*x as i32, *y as i32);
                self.last_pointer_pos = Some((monitor_id.clone(), pos));
                let hit = render_tree.hit_test(pos);
                let hit_cmd = hit.iter().rev().find_map(|n| n.on_click());

                if let Some(cmd) = hit_cmd {
                    if let AppCommand::ScriptCall(func_name) = cmd {
                        actions.push(PointerAction::CallFunction(func_name.clone()));
                    } else {
                        actions.push(PointerAction::SendCommand(cmd.clone()));
                    }
                }
            }
            PointerEvent::PointerMotion { x, y } => {
                let pos = Position::new(*x as i32, *y as i32);
                self.last_pointer_pos = Some((monitor_id.clone(), pos));
                let hit = render_tree.hit_test(pos);
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
            PointerEvent::PointerEnter => {
                // Pointer entered module bounds; no action required
            }
            PointerEvent::PointerLeave => {
                self.last_pointer_pos = None;
                if self.last_tooltip.is_some() {
                    actions.push(PointerAction::SendCommand(AppCommand::HideTooltip));
                    self.last_tooltip = None;
                }
            }
            _ => {}
        }

        actions
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
    use crate::shared::primitives::geometry::{Rect, Size};

    fn make_test_tree(
        on_click: Option<AppCommand>,
        on_hover: Option<AppCommand>,
        tooltip: Option<StyledNode>,
    ) -> RenderNode {
        RenderNode::Rect {
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

        let actions = handler.handle_event(
            &PointerEvent::Click {
                x: 10.0,
                y: 10.0,
                button: 1,
            },
            &mon,
            &tree,
        );

        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0], PointerAction::SendCommand(AppCommand::RequestRender));
    }

    #[test]
    fn test_click_script_call_returns_call_function() {
        let mut handler = PointerHandler::new();
        let mon = MonitorId::new("DP-1");
        let func_name = FunctionName::new("toggle_menu");
        let tree = make_test_tree(
            Some(AppCommand::ScriptCall(func_name.clone())),
            None,
            None,
        );

        let actions = handler.handle_event(
            &PointerEvent::Click {
                x: 10.0,
                y: 10.0,
                button: 1,
            },
            &mon,
            &tree,
        );

        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0], PointerAction::CallFunction(func_name));
    }

    #[test]
    fn test_motion_with_tooltip_lifecycle() {
        let mut handler = PointerHandler::new();
        let mon = MonitorId::new("DP-1");
        let tooltip_node = StyledNode::Text {
            text: crate::features::vdom::domain::TextContent::new("hello".to_string()),
            style: ComputedStyle::default(),
            on_click: None,
            on_hover: None,
            tooltip: None,
        };
        let tree = make_test_tree(None, None, Some(tooltip_node.clone()));

        // Motion inside -> ShowTooltip
        let actions = handler.handle_event(
            &PointerEvent::PointerMotion { x: 10.0, y: 10.0 },
            &mon,
            &tree,
        );
        assert_eq!(actions.len(), 1);
        match &actions[0] {
            PointerAction::SendCommand(AppCommand::ShowTooltip { layout }) => {
                assert_eq!(**layout, tooltip_node);
            }
            _ => panic!("Expected ShowTooltip"),
        }

        // Motion again on same tooltip -> no new ShowTooltip action
        let actions2 = handler.handle_event(
            &PointerEvent::PointerMotion { x: 12.0, y: 12.0 },
            &mon,
            &tree,
        );
        assert!(actions2.is_empty());

        // PointerLeave -> HideTooltip
        let actions3 = handler.handle_event(&PointerEvent::PointerLeave, &mon, &tree);
        assert_eq!(actions3.len(), 1);
        assert_eq!(actions3[0], PointerAction::SendCommand(AppCommand::HideTooltip));
        assert!(handler.last_tooltip().is_none());
    }

    #[test]
    fn test_update_after_render_detects_tooltip_change() {
        let mut handler = PointerHandler::new();
        let mon = MonitorId::new("DP-1");
        let tree1 = make_test_tree(None, None, None);

        // Move to (10, 10)
        let _ = handler.handle_event(
            &PointerEvent::PointerMotion { x: 10.0, y: 10.0 },
            &mon,
            &tree1,
        );

        // Render update adds tooltip under pointer
        let tooltip_node = StyledNode::Text {
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
}
