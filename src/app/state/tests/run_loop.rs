//! How the app's run loop routes every wakeup to its handler, and how it ends.

use super::fakes::{CallLog, RecordingDisplay, RecordingSni};
use super::fixtures::{
    Harness, Queues, TestApp, bus_with, config_sampling_every_2500_ms, drain, harness,
    systray_action,
};
use crate::app::commands::SystemCommand;
use crate::app::state::AppError;
use crate::app::test_fakes::FakeRegistry;
use crate::features::layout_engine::domain::DisplayCommand;
use crate::features::module_runtime::ports::LayoutEvent;
use crate::shared::config::domain::Config;
use crate::shared::events::signals::{HyprlandState, SignalHub};
use crate::shared::primitives::geometry::{Scale, Size};
use crate::shared::primitives::{ModuleId, ModuleName, MonitorId};
use std::collections::{BTreeMap, HashMap};

/// Queues a render request, a systray action and a reload of `clock`.
async fn queue_a_display_ui_and_system_command(queues: &Queues) {
    let display = DisplayCommand::RequestRender;
    queues.display.send(display).await.unwrap();
    queues
        .interface
        .send(systray_action("nm", "Activate"))
        .await
        .unwrap();
    let reload = SystemCommand::ReloadModule(ModuleName::new("clock"));
    queues.system.send(reload).await.unwrap();
}

/// Changes the configuration to one sampling metrics every 2.5 s, the focus to `DP-3`, and the
/// monitors to `eDP-1` alone.
fn change_every_signal(hub: &SignalHub) {
    hub.config_tx()
        .send(config_sampling_every_2500_ms())
        .unwrap();
    let monitor = crate::features::workspaces::domain::MonitorName::new("DP-3");
    let focused = HyprlandState::new(BTreeMap::new(), BTreeMap::new(), Some(monitor));
    hub.hyprland_tx().send(focused).unwrap();
    let scales = HashMap::from([(MonitorId::new("eDP-1"), Scale::new(1.0))]);
    hub.monitor_scales_tx().send(scales).unwrap();
}

/// The metrics sampling interval the app's read model holds, in milliseconds.
fn sampling_interval_ms(app: &TestApp) -> u64 {
    app.read_model.config.metrics().update_interval_ms().value()
}

/// A monitor the read model lays modules out on that Wayland no longer reports.
const DEAD_MONITOR: &str = "HDMI-A-1";

/// An app with a command queued on its display, UI and system channels, every signal changed,
/// and a layout kept for `DEAD_MONITOR`; with the handle on the registry's calls from then on.
async fn harness_with_every_wakeup_pending() -> (Harness, CallLog) {
    let registry = FakeRegistry::default();
    let calls = registry.calls.clone();
    let mut harness = harness(Config::default(), registry);
    drain(&calls);
    let layouts = &mut harness.app.read_model.computed_layouts;
    layouts.insert(MonitorId::new(DEAD_MONITOR), HashMap::new());
    queue_a_display_ui_and_system_command(&harness.queues).await;
    change_every_signal(&harness.hub);
    (harness, calls)
}

/// Every command queued on the four channels and every signal change reaches its handler,
/// and the loop ends with the display's error once nothing else wakes it.
#[test]
fn test_run_routes_every_wakeup_to_its_handler() {
    crate::test_utils::block_on(async {
        let (mut harness, calls) = harness_with_every_wakeup_pending().await;
        let dead = MonitorId::new(DEAD_MONITOR);
        let (display, sni) = (RecordingDisplay::new(), RecordingSni::new());
        let (renders, actions) = (display.calls(), sni.calls());

        let bus = bus_with(&harness.hub, sni);
        let result = harness.app.run(display, bus).await;

        let message = "Internal error: idle";
        assert!(matches!(result, Err(AppError::Internal { message: got }) if got == message));
        let mut registry_calls = drain(&calls);
        registry_calls.sort();
        assert_eq!(
            registry_calls,
            ["clear", "load", "reload clock", "spawn_all"]
        );
        assert_eq!(drain(&actions).len(), 1);
        assert!(drain(&renders).contains(&"render".to_string()));
        assert_eq!(harness.app.focused_monitor, "DP-3");
        assert_eq!(sampling_interval_ms(&harness.app), 2500);
        assert!(!harness.app.read_model.computed_layouts.contains_key(&dead));
    });
}

/// A layout event queued for the loop reaches the read model.
#[test]
fn test_run_routes_a_layout_event_to_the_read_model() {
    crate::test_utils::block_on(async {
        let mut harness = harness(Config::default(), FakeRegistry::default());
        let size_change = LayoutEvent::ModuleSizeChanged {
            monitor_id: MonitorId::new("DP-1"),
            module_id: ModuleId::new(4),
            size: Size::new(10, 10),
            settled_width: None,
        };
        harness.queues.layout.send(size_change).await.unwrap();

        let bus = bus_with(&harness.hub, RecordingSni::new());
        let result = harness.app.run(RecordingDisplay::new(), bus).await;

        assert!(result.is_err());
        let sizes = &harness.app.read_model.module_sizes;
        let on_dp1 = sizes.get(&MonitorId::new("DP-1"));
        assert!(on_dp1.is_some_and(|sizes| sizes.contains_key(&ModuleId::new(4))));
    });
}

/// After the display wakes the loop, its pending events are dispatched, and a dispatch failure
/// ends the loop with that error.
#[test]
fn test_run_ends_with_the_error_of_a_failed_dispatch() {
    crate::test_utils::block_on(async {
        let mut harness = harness(Config::default(), FakeRegistry::default());
        let display = RecordingDisplay::failing();
        let calls = display.calls();

        let bus = bus_with(&harness.hub, RecordingSni::new());
        let result = harness.app.run(display, bus).await;

        let message = "Internal error: no compositor";
        assert!(matches!(result, Err(AppError::Internal { message: got }) if got == message));
        assert_eq!(drain(&calls), ["dispatch"]);
    });
}
