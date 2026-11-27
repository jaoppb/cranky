//! What the run loop does with the interactions it polls: the script calls, refreshes and renders
//! each one leads to.
use super::fixture::MODULE;
use super::run_harness::{SUBS, click, clickable, settle, start};
use crate::features::module_runtime::test_support::RecordingModulePort;
use crate::shared::events::core::{
    InteractionEvent, PointerButton, PointerEvent, SurfaceKind, SurfaceLifecycleEvent,
};
use crate::shared::primitives::geometry::Position;
/// A click calls the script on its monitor, then refreshes with the subscriptions and renders.
#[test]
fn a_click_calls_the_script_then_refreshes_and_renders() {
    crate::test_utils::block_on(async {
        let running = start(RecordingModulePort::new(clickable(), SUBS.to_vec())).await;
        running.take();
        running.interact(MODULE, click());
        settle().await;
        let log = running.take();
        let toggle = ("toggle".to_string(), vec!["DP-1".to_string()]);
        assert_eq!(log.calls, vec![toggle]);
        assert_eq!((log.refreshes, log.renders), (vec![SUBS.to_vec()], 1));
    });
}

/// A failed call changes nothing a click would render, so there is neither refresh nor render.
#[test]
fn a_click_whose_call_fails_neither_refreshes_nor_renders() {
    crate::test_utils::block_on(async {
        let port = RecordingModulePort::new(clickable(), SUBS.to_vec()).failing_calls("boom");
        let running = start(port).await;
        running.take();
        running.interact(MODULE, click());
        settle().await;
        let log = running.take();
        assert_eq!(log.calls.len(), 1);
        assert_eq!((log.refreshes, log.renders), (vec![], 0));
    });
}

/// A press changes the pointer state only: a render, no call and no refresh. A dismissed popup
/// calls its handler, refreshes and renders.
#[test]
fn a_press_renders_and_a_dismissed_popup_calls_its_handler_and_renders() {
    crate::test_utils::block_on(async {
        let running = start(RecordingModulePort::new(clickable(), SUBS.to_vec())).await;
        running.take();
        let press = PointerEvent::ButtonPress {
            surface: SurfaceKind::Bar,
            button: PointerButton::Left,
            pos: Position::new(5, 5),
        };
        running.interact(MODULE, InteractionEvent::Pointer(press));
        settle().await;
        let log = running.take();
        assert_eq!((log.calls, log.refreshes, log.renders), (vec![], vec![], 1));

        let dismissed = InteractionEvent::Lifecycle(SurfaceLifecycleEvent::PopupDismissed);
        running.interact(MODULE, dismissed);
        settle().await;
        let log = running.take();
        let handler = ("on_popup_dismiss".to_string(), vec!["DP-1".to_string()]);
        assert_eq!(log.calls, vec![handler]);
        assert_eq!((log.refreshes, log.renders), (vec![SUBS.to_vec()], 1));
    });
}
