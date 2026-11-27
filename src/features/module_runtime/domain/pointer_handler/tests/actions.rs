//! The branches of pointer handling the lifecycle tests do not reach: hover actions, leaving or
//! pressing outside every node, tray clicks carrying the pointer position, and events the handler
//! ignores.
use crate::features::layout_engine::domain::{NodePath, RenderNode, SurfaceSpace};
use crate::features::module_runtime::domain::pointer_handler::action::PointerAction;
use crate::features::module_runtime::domain::pointer_handler::handler::PointerHandler;
use crate::features::module_runtime::domain::pointer_handler::target::PointerTarget;
use crate::features::styling::domain::ComputedStyle;
use crate::features::systray::domain::{SystrayActionName, SystrayId};
use crate::features::vdom::domain::{ClickHandlers, NodeRef, UiAction, UiCommand};
use crate::shared::events::core::{
    PointerButton, PointerEvent, ScrollAxis, ScrollDelta, SurfaceKind,
};
use crate::shared::primitives::geometry::{Position, Rect, Size};
use crate::shared::primitives::{FunctionName, MonitorId};

/// A 120x30 node at (5, 2) with the given hover and click actions.
fn tree(on_hover: Option<UiAction>, on_click: Option<ClickHandlers>) -> RenderNode {
    RenderNode::Rect {
        path: NodePath::root_in(SurfaceSpace::Bar),
        node_key: None,
        rect: Rect::new(Position::new(5, 2), Size::new(120, 30)),
        style: ComputedStyle::default(),
        on_click,
        on_hover,
        tooltip: None,
        popup: None,
        panel: None,
    }
}

fn root_ref() -> NodeRef {
    NodeRef::new(Vec::new(), NodePath::root_in(SurfaceSpace::Bar))
}

fn tray(name: &str) -> UiAction {
    UiAction::SystrayAction {
        id: SystrayId::new("nm-applet"),
        action: SystrayActionName::parse_str(name),
        pos: None,
    }
}

fn motion(x: i32, y: i32) -> PointerEvent {
    PointerEvent::PointerMotion {
        surface: SurfaceKind::Bar,
        pos: Position::new(x, y),
    }
}

fn press(x: i32, y: i32) -> PointerEvent {
    PointerEvent::ButtonPress {
        surface: SurfaceKind::Bar,
        button: PointerButton::Left,
        pos: Position::new(x, y),
    }
}

fn release(x: i32, y: i32) -> PointerEvent {
    PointerEvent::ButtonRelease {
        surface: SurfaceKind::Bar,
        button: PointerButton::Left,
        pos: Position::new(x, y),
    }
}

#[test]
fn hovering_a_node_runs_its_hover_action_for_this_monitor() {
    let mon = MonitorId::new("HDMI-A-1");
    let call = tree(Some(UiAction::ScriptCall(FunctionName::new("peek"))), None);
    let outcome =
        PointerHandler::new().handle_event(&motion(40, 11), PointerTarget::new(&mon, &call));
    assert_eq!(
        outcome.into_actions(),
        vec![PointerAction::CallFunction(
            FunctionName::new("peek"),
            Some(mon.clone())
        )]
    );

    let exec = tree(Some(UiAction::Exec("notify-send hi".into())), None);
    let outcome =
        PointerHandler::new().handle_event(&motion(40, 11), PointerTarget::new(&mon, &exec));
    assert_eq!(
        outcome.into_actions(),
        vec![PointerAction::SendUi(UiCommand::Exec(
            "notify-send hi".into()
        ))]
    );
}

/// The tray action `name` of nm-applet, sent with the pointer at `pos`.
fn tray_sent(name: &str, pos: Position) -> PointerAction {
    PointerAction::SendUi(UiCommand::SystrayAction {
        id: SystrayId::new("nm-applet"),
        action: SystrayActionName::parse_str(name),
        pos: Some(pos),
    })
}

/// A tray action is sent with the pointer position of the event, not the one in the script.
#[test]
fn tray_hover_and_click_carry_the_pointer_position() {
    let mon = MonitorId::new("DP-1");
    let hover = tree(Some(tray("Activate")), None);
    let outcome =
        PointerHandler::new().handle_event(&motion(40, 11), PointerTarget::new(&mon, &hover));
    assert_eq!(
        outcome.into_actions(),
        vec![tray_sent("Activate", Position::new(40, 11))]
    );

    let click = tree(None, Some(ClickHandlers::from_single(tray("ContextMenu"))));
    let right_click = PointerEvent::Click {
        surface: SurfaceKind::Bar,
        button: PointerButton::Right,
        pos: Position::new(73, 19),
    };
    let outcome =
        PointerHandler::new().handle_event(&right_click, PointerTarget::new(&mon, &click));
    assert_eq!(
        outcome.into_actions(),
        vec![tray_sent("ContextMenu", Position::new(73, 19))]
    );
}

#[test]
fn moving_off_every_node_clears_the_hover_of_that_monitor_only() {
    let mut handler = PointerHandler::new();
    let mon = MonitorId::new("DP-1");
    let other = MonitorId::new("HDMI-A-1");
    let node = tree(None, None);
    let _ = handler.handle_event(&motion(40, 11), PointerTarget::new(&mon, &node));
    let _ = handler.handle_event(&motion(40, 11), PointerTarget::new(&other, &node));

    let outcome = handler.handle_event(&motion(300, 11), PointerTarget::new(&mon, &node));
    assert!(outcome.has_state_changed());
    assert_eq!(outcome.into_actions(), vec![]);
    assert_eq!(handler.hovered_node(&mon), None);
    assert_eq!(handler.hovered_node(&other), Some(&root_ref()));
}

#[test]
fn pressing_outside_every_node_clears_the_press_once() {
    let mut handler = PointerHandler::new();
    let mon = MonitorId::new("DP-1");
    let node = tree(None, None);
    let mut changed = |event: PointerEvent| {
        handler
            .handle_event(&event, PointerTarget::new(&mon, &node))
            .has_state_changed()
    };
    assert!(changed(press(40, 11)));
    assert!(!changed(press(41, 12)));
    assert!(changed(press(2, 40)));
    assert!(!changed(press(2, 40)));
    assert_eq!(handler.active_node(&mon), None);
}

#[test]
fn releasing_outside_every_node_drops_the_focus() {
    let mut handler = PointerHandler::new();
    let mon = MonitorId::new("DP-1");
    let node = tree(None, None);
    let mut changed = |event: PointerEvent| {
        handler
            .handle_event(&event, PointerTarget::new(&mon, &node))
            .has_state_changed()
    };
    assert!(changed(release(40, 11)));
    assert!(!changed(release(41, 12)));
    assert!(changed(release(2, 40)));
    assert_eq!(handler.focused_node(&mon), None);
}

#[test]
fn enter_and_scroll_events_do_nothing() {
    let mut handler = PointerHandler::new();
    let mon = MonitorId::new("DP-1");
    let node = tree(Some(tray("Activate")), None);
    let enter = PointerEvent::PointerEnter {
        surface: SurfaceKind::Bar,
    };
    let scroll = PointerEvent::Scroll {
        surface: SurfaceKind::Bar,
        axis: ScrollAxis::Vertical,
        amount: ScrollDelta::new(-1.0),
    };
    for event in [enter, scroll] {
        let outcome = handler.handle_event(&event, PointerTarget::new(&mon, &node));
        assert!(!outcome.has_state_changed());
        assert_eq!(outcome.into_actions(), vec![]);
    }
    assert_eq!(handler.hovered_node(&mon), None);
}
