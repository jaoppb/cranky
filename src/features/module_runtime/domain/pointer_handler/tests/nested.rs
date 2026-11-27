//! Which node's action a click or a hover runs when the pointer is over a node and its parent:
//! the innermost node with an action, falling back to its ancestors.
use crate::features::layout_engine::domain::{NodePath, RenderNode, SurfaceSpace};
use crate::features::module_runtime::domain::pointer_handler::action::PointerAction;
use crate::features::module_runtime::domain::pointer_handler::handler::PointerHandler;
use crate::features::module_runtime::domain::pointer_handler::target::PointerTarget;
use crate::features::styling::domain::ComputedStyle;
use crate::features::vdom::domain::{ClickHandlers, UiAction, UiCommand};
use crate::shared::events::core::{PointerButton, PointerEvent, SurfaceKind};
use crate::shared::primitives::MonitorId;
use crate::shared::primitives::geometry::{Position, Rect, Size};

/// What a node runs when hovered, and when clicked with any button.
type Actions = (Option<&'static str>, Option<&'static str>);

fn exec(command: &str) -> UiAction {
    UiAction::Exec(command.into())
}

fn sent(command: &str) -> PointerAction {
    PointerAction::SendUi(UiCommand::Exec(command.into()))
}

/// A 100x30 bar running `parent` that holds, at (10, 0), a 20x20 child running `child`.
fn bar(parent: Actions, child: Actions) -> RenderNode {
    let (child_hover, child_click) = child;
    let inner = RenderNode::Rect {
        path: NodePath::new(SurfaceSpace::Bar, vec![0]),
        node_key: None,
        rect: Rect::new(Position::new(10, 0), Size::new(20, 20)),
        style: ComputedStyle::default(),
        on_click: child_click.map(|command| ClickHandlers::from_single(exec(command))),
        on_hover: child_hover.map(exec),
        tooltip: None,
        popup: None,
        panel: None,
    };
    let (parent_hover, parent_click) = parent;
    RenderNode::Flex {
        path: NodePath::root_in(SurfaceSpace::Bar),
        node_key: None,
        rect: Rect::new(Position::new(0, 0), Size::new(100, 30)),
        children: vec![inner],
        style: ComputedStyle::default(),
        on_click: parent_click.map(|command| ClickHandlers::from_single(exec(command))),
        on_hover: parent_hover.map(exec),
        tooltip: None,
        popup: None,
        panel: None,
    }
}

/// A left click over the child.
fn click_child(tree: &RenderNode) -> Vec<PointerAction> {
    let event = PointerEvent::Click {
        surface: SurfaceKind::Bar,
        button: PointerButton::Left,
        pos: Position::new(15, 5),
    };
    let mon = MonitorId::new("DP-1");
    PointerHandler::new()
        .handle_event(&event, PointerTarget::new(&mon, tree))
        .into_actions()
}

/// Moves over the child with `handler`, and returns the actions and whether the state changed.
fn hover_child(handler: &mut PointerHandler, tree: &RenderNode) -> (Vec<PointerAction>, bool) {
    let event = PointerEvent::PointerMotion {
        surface: SurfaceKind::Bar,
        pos: Position::new(15, 5),
    };
    let mon = MonitorId::new("DP-1");
    let outcome = handler.handle_event(&event, PointerTarget::new(&mon, tree));
    let changed = outcome.has_state_changed();
    (outcome.into_actions(), changed)
}

#[test]
fn a_click_runs_the_innermost_handler_and_falls_back_to_the_parent() {
    let both = bar((None, Some("parent")), (None, Some("child")));
    assert_eq!(click_child(&both), vec![sent("child")]);

    let parent_only = bar((None, Some("parent")), (None, None));
    assert_eq!(click_child(&parent_only), vec![sent("parent")]);
}

#[test]
fn a_hover_runs_the_innermost_action_and_falls_back_to_the_parent() {
    let both = bar((Some("parent"), None), (Some("child"), None));
    assert_eq!(
        hover_child(&mut PointerHandler::new(), &both).0,
        vec![sent("child")]
    );

    let parent_only = bar((Some("parent"), None), (None, None));
    assert_eq!(
        hover_child(&mut PointerHandler::new(), &parent_only).0,
        vec![sent("parent")]
    );
}

/// Moving again over the node already hovered changes no state but runs its action again.
#[test]
fn a_hover_action_runs_on_every_motion_over_the_same_node() {
    let tree = bar((None, None), (Some("child"), None));
    let mut handler = PointerHandler::new();
    assert_eq!(
        hover_child(&mut handler, &tree),
        (vec![sent("child")], true)
    );
    assert_eq!(
        hover_child(&mut handler, &tree),
        (vec![sent("child")], false)
    );
}
