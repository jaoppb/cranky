use crate::features::layout_engine::domain::{
    DisplayCommand, FloatingKind, NodePath, RenderNode, StyledNode,
};
use crate::features::module_runtime::domain::pointer_handler::action::{
    PointerAction, PointerOutcome,
};
use crate::features::module_runtime::domain::pointer_handler::handler::PointerHandler;
use crate::features::styling::domain::ComputedStyle;
use crate::features::vdom::domain::TextContent;
use crate::shared::events::core::{PointerButton, PointerEvent};
use crate::shared::primitives::geometry::{Position, Rect, Size};
use crate::shared::primitives::{FunctionName, MonitorId};
use std::collections::HashMap;

fn make_test_tree(tooltip: Option<StyledNode>) -> RenderNode {
    RenderNode::Rect {
        path: NodePath::root(),
        rect: Rect::new(Position::new(0, 0), Size::new(100, 100)),
        style: ComputedStyle::default(),
        on_click: None,
        on_hover: None,
        tooltip: tooltip.map(Box::new),
        popup: None,
    }
}

#[test]
fn test_interaction_state_lifecycle() {
    let mut handler = PointerHandler::new();
    let mon = MonitorId::new("DP-1");
    let tree = make_test_tree(None);

    let outcome_motion = handler.handle_event(
        &PointerEvent::PointerMotion {
            pos: Position::new(10, 10),
        },
        &mon,
        &tree,
    );
    assert!(outcome_motion.has_state_changed());
    assert_eq!(handler.hovered_node(&mon), Some(&NodePath::root()));

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

    let outcome_leave = handler.handle_event(&PointerEvent::PointerLeave, &mon, &tree);
    assert!(outcome_leave.has_state_changed());
    assert_eq!(handler.hovered_node(&mon), None);
    assert_eq!(handler.focused_node(&mon), Some(&NodePath::root()));
}

#[test]
fn test_motion_with_tooltip_lifecycle() {
    let mut handler = PointerHandler::new();
    let mon = MonitorId::new("DP-1");
    let tooltip_node = StyledNode::Text {
        path: NodePath::root(),
        text: TextContent::new("hello".to_string()),
        style: ComputedStyle::default(),
        on_click: None,
        on_hover: None,
        tooltip: None,
        popup: None,
    };
    let tree = make_test_tree(Some(tooltip_node.clone()));

    let outcome = handler.handle_event(
        &PointerEvent::PointerMotion {
            pos: Position::new(10, 10),
        },
        &mon,
        &tree,
    );
    assert_eq!(outcome.actions().len(), 1);
    match &outcome.actions()[0] {
        PointerAction::SendDisplay(DisplayCommand::ShowFloatingSurface {
            kind, layout, ..
        }) => {
            assert_eq!(*kind, FloatingKind::Tooltip);
            assert_eq!(**layout, tooltip_node);
        }
        _ => panic!("Expected ShowFloatingSurface"),
    }

    let outcome2 = handler.handle_event(
        &PointerEvent::PointerMotion {
            pos: Position::new(12, 12),
        },
        &mon,
        &tree,
    );
    assert!(outcome2.actions().is_empty());

    let outcome3 = handler.handle_event(&PointerEvent::PointerLeave, &mon, &tree);
    assert_eq!(outcome3.actions().len(), 1);
    assert_eq!(
        outcome3.actions()[0],
        PointerAction::SendDisplay(DisplayCommand::HideFloatingSurface {
            kind: FloatingKind::Tooltip,
        })
    );
    assert!(handler.last_tooltip().is_none());
}

#[test]
fn test_update_after_render_detects_tooltip_change() {
    let mut handler = PointerHandler::new();
    let mon = MonitorId::new("DP-1");
    let tree1 = make_test_tree(None);

    let _ = handler.handle_event(
        &PointerEvent::PointerMotion {
            pos: Position::new(10, 10),
        },
        &mon,
        &tree1,
    );

    let tooltip_node = StyledNode::Text {
        path: NodePath::root(),
        text: TextContent::new("dynamic".to_string()),
        style: ComputedStyle::default(),
        on_click: None,
        on_hover: None,
        tooltip: None,
        popup: None,
    };
    let tree2 = make_test_tree(Some(tooltip_node));
    let mut render_trees = HashMap::new();
    render_trees.insert(mon.clone(), tree2);

    let actions = handler.update_after_render(&render_trees);
    assert_eq!(actions.len(), 1);
}

#[test]
fn test_popup_dismissed_event() {
    let mut handler = PointerHandler::new();
    let mon = MonitorId::new("DP-1");
    let tree = make_test_tree(None);

    let outcome = handler.handle_event(&PointerEvent::PopupDismissed, &mon, &tree);
    assert!(outcome.has_state_changed());
    assert_eq!(
        outcome.into_actions(),
        vec![PointerAction::CallFunction(
            FunctionName::new("on_popup_dismiss"),
            Some(mon)
        )]
    );
}

#[test]
fn test_pointer_outcome_methods() {
    let empty = PointerOutcome::empty();
    assert!(empty.actions().is_empty());
    assert!(!empty.has_state_changed());
    assert_eq!(empty.into_actions(), Vec::<PointerAction>::new());
}
