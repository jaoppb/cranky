use crate::features::layout_engine::domain::{NodePath, RenderNode};
use crate::features::module_runtime::domain::pointer_handler::action::PointerAction;
use crate::features::module_runtime::domain::pointer_handler::handler::PointerHandler;
use crate::features::styling::domain::ComputedStyle;
use crate::features::vdom::domain::{ClickHandlers, UiAction, UiCommand};
use crate::shared::events::core::{PointerButton, PointerEvent, SurfaceKind};
use crate::shared::primitives::geometry::{Position, Rect, Size};
use crate::shared::primitives::{FunctionName, MonitorId};

fn make_test_tree(on_click: Option<ClickHandlers>) -> RenderNode {
    RenderNode::Rect {
        path: NodePath::root(),
        node_key: None,
        rect: Rect::new(Position::new(0, 0), Size::new(100, 100)),
        style: ComputedStyle::default(),
        on_click,
        on_hover: None,
        tooltip: None,
        popup: None,
        panel: None,
    }
}

#[test]
fn test_click_hit_returns_send_command_single_shorthand() {
    let mut handler = PointerHandler::new();
    let mon = MonitorId::new("DP-1");
    let tree = make_test_tree(Some(ClickHandlers::from_single(UiAction::Exec(
        "render".into(),
    ))));

    // Left click
    let outcome = handler.handle_event(
        &PointerEvent::Click {
            surface: SurfaceKind::Bar,
            button: PointerButton::Left,
            pos: Position::new(10, 10),
        },
        &mon,
        &tree,
    );
    assert_eq!(outcome.actions().len(), 1);
    assert_eq!(
        outcome.actions()[0],
        PointerAction::SendUi(UiCommand::Exec("render".into()))
    );

    // Right click also triggers on single action shorthand
    let outcome_right = handler.handle_event(
        &PointerEvent::Click {
            surface: SurfaceKind::Bar,
            button: PointerButton::Right,
            pos: Position::new(10, 10),
        },
        &mon,
        &tree,
    );
    assert_eq!(outcome_right.actions().len(), 1);
    assert_eq!(
        outcome_right.actions()[0],
        PointerAction::SendUi(UiCommand::Exec("render".into()))
    );
}

#[test]
fn test_click_hit_returns_distinct_button_actions() {
    let mut handler = PointerHandler::new();
    let mon = MonitorId::new("DP-1");
    let mut handlers = ClickHandlers::new();
    handlers.insert(PointerButton::Left, UiAction::Exec("left_clicked".into()));
    handlers.insert(PointerButton::Right, UiAction::Exec("right_clicked".into()));
    handlers.insert(PointerButton::Side, UiAction::Exec("side_clicked".into()));

    let tree = make_test_tree(Some(handlers));

    let outcome_left = handler.handle_event(
        &PointerEvent::Click {
            surface: SurfaceKind::Bar,
            button: PointerButton::Left,
            pos: Position::new(10, 10),
        },
        &mon,
        &tree,
    );
    assert_eq!(
        outcome_left.into_actions(),
        vec![PointerAction::SendUi(UiCommand::Exec(
            "left_clicked".into()
        ))]
    );

    let outcome_right = handler.handle_event(
        &PointerEvent::Click {
            surface: SurfaceKind::Bar,
            button: PointerButton::Right,
            pos: Position::new(10, 10),
        },
        &mon,
        &tree,
    );
    assert_eq!(
        outcome_right.into_actions(),
        vec![PointerAction::SendUi(UiCommand::Exec(
            "right_clicked".into()
        ))]
    );
}

#[test]
fn test_click_script_call_returns_call_function() {
    let mut handler = PointerHandler::new();
    let mon = MonitorId::new("DP-1");
    let func_name = FunctionName::new("toggle_menu");
    let tree = make_test_tree(Some(ClickHandlers::from_single(UiAction::ScriptCall(
        func_name.clone(),
    ))));

    let outcome = handler.handle_event(
        &PointerEvent::Click {
            surface: SurfaceKind::Bar,
            button: PointerButton::Left,
            pos: Position::new(10, 10),
        },
        &mon,
        &tree,
    );

    assert_eq!(outcome.actions().len(), 1);
    assert_eq!(
        outcome.into_actions(),
        vec![PointerAction::CallFunction(func_name, Some(mon))]
    );
}
