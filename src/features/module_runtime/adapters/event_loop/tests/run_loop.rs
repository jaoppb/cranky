//! The actor's run loop: what the signals, bounds and child sizes it polls refresh and render
//! again, and when it stops.
use super::fixture::MODULE;
use super::run_harness::{Running, SUBS, click, clickable, kinds, settle, start};
use crate::features::module_runtime::test_support::{PortLog, RecordingModulePort, capture_logs};
use crate::shared::events::signals::HyprlandState;
use crate::shared::primitives::geometry::{Position, Rect, Size};
use crate::shared::primitives::{ChildBounds, ChildSizesMap, ModuleId, MonitorId, SizeConstraint};
use std::collections::{BTreeMap, HashMap, HashSet};
/// The loop refreshes with every subscription and renders before it polls; the subscriptions'
/// first values then arrive as one burst, as do signals sent together later.
#[test]
fn startup_refreshes_and_renders_then_each_burst_of_signals_refreshes_once() {
    crate::test_utils::block_on(async {
        let running = start(RecordingModulePort::new(clickable(), SUBS.to_vec())).await;
        let started = running.take();
        assert_eq!(started.refreshes.first(), Some(&SUBS.to_vec()));
        assert_eq!(kinds(&started.refreshes), vec![HashSet::from(SUBS); 2]);
        assert_eq!(started.renders, 2);

        let _ = running
            .wiring
            .hub
            .time_tx()
            .send(cranky_time::LocalTime::utc(0));
        let hyprland = HyprlandState::new(BTreeMap::new(), BTreeMap::new(), None);
        running.wiring.hub.hyprland_tx().send_replace(hyprland);
        settle().await;
        let burst = running.take();
        assert_eq!(kinds(&burst.refreshes), vec![HashSet::from(SUBS)]);
        assert_eq!(burst.renders, 1);
    });
}

/// An interaction meant for another module wakes the loop with no signal: no refresh, no render.
#[test]
fn an_interaction_for_another_module_neither_refreshes_nor_renders() {
    crate::test_utils::block_on(async {
        let running = start(RecordingModulePort::new(clickable(), SUBS.to_vec())).await;
        running.take();
        running.interact(ModuleId::new(7), click());
        settle().await;
        assert_eq!(running.take(), PortLog::default());
    });
}

/// New bounds from the parent refresh the script with its subscriptions and render it.
#[test]
fn a_layout_change_refreshes_with_the_subscriptions_and_renders() {
    crate::test_utils::block_on(async {
        let running = start(RecordingModulePort::new(clickable(), SUBS.to_vec())).await;
        running.take();
        let bounds = ChildBounds::new(
            Rect::new(Position::new(0, 0), Size::new(10, 10)),
            SizeConstraint::none(),
        );
        let _ = running
            .wiring
            .layouts
            .send(HashMap::from([(MonitorId::new("DP-1"), bounds)]));
        settle().await;
        let log = running.take();
        assert_eq!(log.refreshes, vec![SUBS.to_vec()]);
        assert_eq!(log.renders, 1);
    });
}

/// Child sizes render again, without a refresh, only when they differ from those the last render
/// on some monitor used.
#[test]
fn child_sizes_render_only_when_they_differ_from_the_last_render() {
    crate::test_utils::block_on(async {
        let running = start(RecordingModulePort::new(clickable(), SUBS.to_vec())).await;
        running.take();
        let sizes =
            |monitor: &str| HashMap::from([(MonitorId::new(monitor), ChildSizesMap::new())]);
        let _ = running.wiring.hub.module_sizes_tx().send(sizes("GHOST-1"));
        settle().await;
        assert_eq!(running.take(), PortLog::default());

        let _ = running.wiring.hub.module_sizes_tx().send(sizes("DP-1"));
        settle().await;
        let log = running.take();
        assert_eq!((log.refreshes, log.renders), (vec![], 1));
    });
}

/// The loop ends once the watch the parent sends bounds through closes.
#[test]
fn the_loop_stops_when_the_layout_watch_closes() {
    crate::test_utils::block_on(async {
        let Running { wiring, task, .. } =
            start(RecordingModulePort::new(clickable(), SUBS.to_vec())).await;
        drop(wiring.layouts);
        let stopped = tokio::time::timeout(std::time::Duration::from_secs(5), task).await;
        assert!(matches!(stopped, Ok(Ok(()))));
    });
}

/// Interactions the loop fell too far behind on are logged as skipped, and the loop goes on.
#[test]
fn a_lagging_interaction_channel_is_logged_and_the_loop_goes_on() {
    let logs = capture_logs(|| {
        crate::test_utils::block_on(async {
            let running = start(RecordingModulePort::new(clickable(), SUBS.to_vec())).await;
            (0..40).for_each(|_| running.interact(ModuleId::new(7), click()));
            settle().await;
            running.take();
            running.interact(MODULE, click());
            settle().await;
            assert_eq!(running.take().calls.len(), 1);
        });
    });
    assert!(logs.contains(
        "WARN cranky::features::module_runtime::adapters::event_loop::poll: ModuleActor input_rx \
         lagged, skipped messages module=42 lagged=8"
    ));
}
