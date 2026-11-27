//! How the app's run loop routes every wakeup to its handler, and how it ends.

use super::fakes::{RecordingDisplay, RecordingSni};
use super::fixtures::{bus_with, config_sampling_every_2500_ms, drain, harness, systray_action};
use crate::app::commands::SystemCommand;
use crate::app::state::AppError;
use crate::app::test_fakes::FakeRegistry;
use crate::features::layout_engine::domain::DisplayCommand;
use crate::shared::config::domain::Config;
use crate::shared::primitives::geometry::Scale;
use crate::shared::primitives::{ModuleId, ModuleName, MonitorId};
use std::collections::{BTreeMap, HashMap};

/// Every command queued on the four channels and every signal change reaches its handler,
/// and the loop ends with the display's error once nothing else wakes it.
#[test]
fn test_run_routes_every_wakeup_to_its_handler() {
    crate::test_utils::block_on(async {
        let registry = FakeRegistry::default();
        let calls = registry.calls.clone();
        let mut harness = harness(Config::default(), registry);
        drain(&calls);
        let dead = MonitorId::new("HDMI-A-1");
        harness
            .app
            .read_model
            .computed_layouts
            .insert(dead.clone(), HashMap::new());
        harness
            .display_tx
            .send(DisplayCommand::RequestRender)
            .await
            .unwrap();
        harness
            .ui_tx
            .send(systray_action("nm", "Activate"))
            .await
            .unwrap();
        harness
            .system_tx
            .send(SystemCommand::ReloadModule(ModuleName::new("clock")))
            .await
            .unwrap();
        let hub = harness.hub.clone();
        hub.config_tx()
            .send(config_sampling_every_2500_ms())
            .unwrap();
        hub.hyprland_tx()
            .send(crate::shared::events::signals::HyprlandState::new(
                BTreeMap::new(),
                BTreeMap::new(),
                Some(crate::features::workspaces::domain::MonitorName::new(
                    "DP-3",
                )),
            ))
            .unwrap();
        hub.monitor_scales_tx()
            .send(HashMap::from([(MonitorId::new("eDP-1"), Scale::new(1.0))]))
            .unwrap();
        let display = RecordingDisplay::new();
        let renders = display.calls();
        let sni = RecordingSni::new();
        let actions = sni.calls();

        let result = harness.app.run(display, bus_with(&hub, sni)).await;

        assert!(
            matches!(result, Err(AppError::Internal { message }) if message == "Internal error: idle")
        );
        let mut registry_calls = drain(&calls);
        registry_calls.sort();
        assert_eq!(
            registry_calls,
            ["clear", "load", "reload clock", "spawn_all"]
        );
        assert_eq!(drain(&actions).len(), 1);
        assert!(drain(&renders).contains(&"render".to_string()));
        assert_eq!(harness.app.focused_monitor, "DP-3");
        assert_eq!(
            harness
                .app
                .read_model
                .config
                .metrics()
                .update_interval_ms()
                .value(),
            2500
        );
        assert!(!harness.app.read_model.computed_layouts.contains_key(&dead));
    });
}

/// A layout event queued for the loop reaches the read model.
#[test]
fn test_run_routes_a_layout_event_to_the_read_model() {
    crate::test_utils::block_on(async {
        let mut harness = harness(Config::default(), FakeRegistry::default());
        harness
            .layout_tx
            .send(
                crate::features::module_runtime::ports::LayoutEvent::ModuleSizeChanged {
                    monitor_id: MonitorId::new("DP-1"),
                    module_id: ModuleId::new(4),
                    size: crate::shared::primitives::geometry::Size::new(10, 10),
                    settled_width: None,
                },
            )
            .await
            .unwrap();
        let hub = harness.hub.clone();

        let result = harness
            .app
            .run(RecordingDisplay::new(), bus_with(&hub, RecordingSni::new()))
            .await;

        assert!(result.is_err());
        assert!(
            harness
                .app
                .read_model
                .module_sizes
                .get(&MonitorId::new("DP-1"))
                .is_some_and(|sizes| sizes.contains_key(&ModuleId::new(4)))
        );
    });
}

/// After the display wakes the loop, its pending events are dispatched, and a dispatch failure
/// ends the loop with that error.
#[test]
fn test_run_ends_with_the_error_of_a_failed_dispatch() {
    crate::test_utils::block_on(async {
        let mut harness = harness(Config::default(), FakeRegistry::default());
        let hub = harness.hub.clone();
        let display = RecordingDisplay::failing();
        let calls = display.calls();

        let result = harness
            .app
            .run(display, bus_with(&hub, RecordingSni::new()))
            .await;

        assert!(
            matches!(result, Err(AppError::Internal { message }) if message == "Internal error: no compositor")
        );
        assert_eq!(drain(&calls), ["dispatch"]);
    });
}
