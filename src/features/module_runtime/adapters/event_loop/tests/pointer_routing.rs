//! Which tree a pointer event is hit-tested against, what a dismissed surface forgets, and what
//! the actions it triggers call, send and log.
use super::fixture::{ChannelEventLoop, Wiring, hub_with_dp1, wired_event_loop};
use crate::features::layout_engine::adapters::taffy::TaffyLayoutAdapter;
use crate::features::layout_engine::domain::{NodePath, RenderNode, SurfaceSpace};
use crate::features::module_runtime::test_support::{PortLog, RecordingModulePort, capture_logs};
use crate::features::styling::domain::ComputedStyle;
use crate::features::vdom::domain::{
    ClickHandlers, ElementSelectors, Interactions, TextContent, UiAction, UiCommand, VNode,
};
use crate::shared::events::core::{
    PointerButton, PointerEvent, SurfaceKind, SurfaceLifecycleEvent,
};
use crate::shared::events::signals::SignalKind;
use crate::shared::primitives::geometry::{Position, Rect, Size};
use crate::shared::primitives::{FunctionName, MonitorId};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// A 50x20 node at the origin of `surface` running `action` when clicked.
fn clickable(surface: SurfaceSpace, action: UiAction) -> RenderNode {
    RenderNode::Rect {
        path: NodePath::root_in(surface),
        node_key: None,
        rect: Rect::new(Position::new(0, 0), Size::new(50, 20)),
        style: ComputedStyle::default(),
        on_click: Some(ClickHandlers::from_single(action)),
        on_hover: None,
        tooltip: None,
        popup: None,
        panel: None,
    }
}

fn call(name: &str) -> UiAction {
    UiAction::ScriptCall(FunctionName::new(name))
}

fn click_on(surface: SurfaceKind) -> PointerEvent {
    PointerEvent::Click {
        surface,
        button: PointerButton::Left,
        pos: Position::new(5, 5),
    }
}

fn dp1() -> MonitorId {
    MonitorId::new("DP-1")
}

/// A port subscribed to time whose 10x10 bar text calls `bar` when clicked.
fn bar_port() -> RecordingModulePort {
    let bar = VNode::new_text(
        TextContent::new("bar".to_string()),
        ElementSelectors::default(),
        Interactions::new(Some(ClickHandlers::from_single(call("bar"))), None, None),
    );
    RecordingModulePort::new(bar, vec![SignalKind::Time])
}

/// An event loop running `port`, rendered once on DP-1, with an open popup calling `popup` and
/// an open panel calling `panel`, and the log of its port.
fn with_three_trees(port: RecordingModulePort) -> (ChannelEventLoop, Wiring, Arc<Mutex<PortLog>>) {
    let log = port.log();
    let (mut event_loop, wiring) = wired_event_loop(port, hub_with_dp1(1.0));
    event_loop.render_all_monitors(&mut HashMap::<MonitorId, TaffyLayoutAdapter>::new());
    let popup = clickable(SurfaceSpace::Popup, call("popup"));
    event_loop.popup_render_trees.insert(dp1(), (None, popup));
    let panel = clickable(SurfaceSpace::Panel, call("panel"));
    event_loop.panel_render_trees.insert(dp1(), (None, panel));
    log.lock().unwrap().renders = 0;
    (event_loop, wiring, log)
}

fn called(log: &Mutex<PortLog>) -> Vec<String> {
    let calls = std::mem::take(&mut log.lock().unwrap().calls);
    calls.into_iter().map(|(name, _)| name).collect()
}

/// Each surface's pointer events hit-test its own tree, and a call that ran changes the module.
#[test]
fn each_surface_routes_the_pointer_to_its_own_tree() {
    let (mut event_loop, _wiring, log) = with_three_trees(bar_port());
    for (surface, expected) in [
        (SurfaceKind::Bar, "bar"),
        (SurfaceKind::Popup, "popup"),
        (SurfaceKind::Panel, "panel"),
    ] {
        assert!(event_loop.handle_pointer_event(&dp1(), &click_on(surface)));
        assert_eq!(called(&log), [expected]);
    }
    let refreshes = std::mem::take(&mut log.lock().unwrap().refreshes);
    assert_eq!(refreshes, vec![vec![SignalKind::Time]; 3]);
}

/// A surface with no tree on the monitor changes nothing, and says so in the log.
#[test]
fn a_surface_with_no_tree_changes_nothing_and_is_logged() {
    let rect = VNode::new_rect(ElementSelectors::default(), Interactions::default());
    let port = RecordingModulePort::new(rect, vec![]);
    let log = port.log();
    let (mut event_loop, _wiring) = wired_event_loop(port, hub_with_dp1(1.0));
    let logs = capture_logs(|| {
        assert!(!event_loop.handle_pointer_event(&dp1(), &click_on(SurfaceKind::Popup)));
    });
    assert_eq!(*log.lock().unwrap(), PortLog::default());
    let target = "cranky::features::module_runtime::adapters::event_loop::pointer";
    assert!(logs.contains(&format!(
        "DEBUG {target}: Received pointer event in module actor module=42 monitor=DP-1 \
         event=Click {{ surface: Popup, button: Left, pos: Position {{ x: 5, y: 5 }} }}"
    )));
    assert!(logs.contains(&format!(
        "WARN {target}: Received pointer event but no render tree found for monitor module=42 \
         monitor=DP-1"
    )));
}

/// A dismissed panel forgets the panel's tree only and calls its handler with the monitor; a
/// dismissed popup then forgets the popup's.
#[test]
fn a_dismissed_surface_forgets_only_its_own_tree_and_calls_its_handler() {
    let (mut event_loop, _wiring, log) = with_three_trees(bar_port());
    let logs = capture_logs(|| {
        assert!(event_loop.handle_lifecycle_event(&dp1(), SurfaceLifecycleEvent::PanelDismissed));
    });
    assert!(event_loop.popup_render_trees.contains_key(&dp1()));
    assert!(!event_loop.panel_render_trees.contains_key(&dp1()));
    let calls = std::mem::take(&mut log.lock().unwrap().calls);
    assert_eq!(
        calls,
        [("on_panel_dismiss".to_string(), vec!["DP-1".to_string()])]
    );
    assert!(logs.contains(
        "DEBUG cranky::features::module_runtime::adapters::event_loop::pointer: Received surface \
         lifecycle event in module actor module=42 monitor=DP-1 event=PanelDismissed"
    ));

    assert!(event_loop.handle_lifecycle_event(&dp1(), SurfaceLifecycleEvent::PopupDismissed));
    assert!(!event_loop.popup_render_trees.contains_key(&dp1()));
    assert_eq!(called(&log), ["on_popup_dismiss"]);
}

/// A failed call is logged and refreshes nothing: a dismissal still changed the pointer state,
/// a click did not.
#[test]
fn a_failed_call_is_logged_and_refreshes_nothing() {
    let (mut event_loop, _wiring, log) = with_three_trees(bar_port().failing_calls("boom"));
    let logs = capture_logs(|| {
        assert!(event_loop.handle_lifecycle_event(&dp1(), SurfaceLifecycleEvent::PopupDismissed));
        assert!(!event_loop.handle_pointer_event(&dp1(), &click_on(SurfaceKind::Panel)));
    });
    assert_eq!(called(&log), ["on_popup_dismiss", "panel"]);
    assert_eq!(log.lock().unwrap().refreshes, Vec::<Vec<SignalKind>>::new());
    assert!(logs.contains(
        "ERROR cranky::features::module_runtime::adapters::event_loop::pointer: ScriptCall \
         failed: Script evaluation error: boom module=42 func=on_popup_dismiss"
    ));
}

/// A UI action goes to the UI sender and changes nothing in the module.
#[test]
fn a_ui_action_is_sent_to_the_ui_and_changes_nothing() {
    let (mut event_loop, wiring, log) = with_three_trees(bar_port());
    let exec = clickable(
        SurfaceSpace::Popup,
        UiAction::Exec("notify-send hi".to_string()),
    );
    event_loop.popup_render_trees.insert(dp1(), (None, exec));
    assert!(!event_loop.handle_pointer_event(&dp1(), &click_on(SurfaceKind::Popup)));
    let sent: Vec<UiCommand> = wiring.sent.ui_commands.try_iter().collect();
    assert_eq!(sent, [UiCommand::Exec("notify-send hi".to_string())]);
    assert_eq!(*log.lock().unwrap(), PortLog::default());
}
