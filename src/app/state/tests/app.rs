//! How the app is built from its registry and configuration, how it reports its errors, and
//! how its run loop ends.

use super::fixtures::{
    Harness, Queues, app_over, bus_with, config_sampling_every_2500_ms, harness, hide_tooltip,
    send_all, show_tooltip, systray_action, try_harness,
};
use crate::app::state::AppError;
use crate::app::test_fakes::{FakeDisplayServer, FakeRegistry, FakeSni};
use crate::features::layout_engine::domain::DisplayCommand;
use crate::features::module_runtime::ports::{LayoutEvent, RegistryLoadError};
use crate::shared::config::domain::Config;
use crate::shared::events::signals::{HyprlandState, SignalHub};
use crate::shared::primitives::geometry::{Scale, Size};
use crate::shared::primitives::{ModuleId, ModuleName, MonitorId};
use crate::shared::wayland::ports::DisplayServerError;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;

#[test]
fn test_app_initialization() {
    crate::test_utils::block_on(async {
        let app_result = try_harness(Config::default(), FakeRegistry::default());

        assert!(app_result.is_ok());
    });
}

/// An app whose registry cannot load the configuration is not built: the load error is
/// returned as a module error.
#[test]
fn test_app_creation_fails_with_the_registry_load_error() {
    crate::test_utils::block_on(async {
        let mut registry = FakeRegistry::default();
        registry.load_failure = Some(ModuleName::new("clock"));

        let app_result = try_harness(Config::default(), registry);

        assert!(matches!(
            app_result,
            Err(AppError::Module(RegistryLoadError::ModuleNotFound(name))) if name.as_str() == "clock"
        ));
    });
}

#[test]
fn test_app_run_exit_on_display_error() {
    crate::test_utils::block_on(async {
        let Harness { mut app, hub, .. } = harness(Config::default(), FakeRegistry::default());

        let result = app
            .run(
                FakeDisplayServer::failing("Test error"),
                bus_with(&hub, FakeSni),
            )
            .await;
        assert!(result.is_err());
    });
}

#[test]
fn test_app_error_fmt() {
    let err1 = AppError::Module(RegistryLoadError::ModuleNotFound("test".into()));
    assert_eq!(err1.to_string(), "Module error: Module not found: test");
    let err2 = AppError::Internal {
        message: "test".into(),
    };
    assert_eq!(err2.to_string(), "Internal error: test");
}

/// Queues one command of every kind `CrankyApp::run` dispatches.
async fn queue_one_of_each_command(queues: &Queues) {
    let size_change = LayoutEvent::ModuleSizeChanged {
        monitor_id: MonitorId::new("1"),
        module_id: ModuleId::new(1),
        size: Size::new(10, 10),
        settled_width: None,
    };
    let display_commands = [
        DisplayCommand::RequestRender,
        show_tooltip(),
        hide_tooltip(),
    ];
    queues.layout.send(size_change).await.unwrap();
    send_all(&queues.display, display_commands).await;
    queues
        .interface
        .send(systray_action("a", "b"))
        .await
        .unwrap();
}

/// Changes the configuration, Hyprland and monitor-scale signals of `hub`.
fn change_every_signal(hub: &SignalHub) {
    hub.config_tx().send(Config::default()).unwrap();
    hub.hyprland_tx()
        .send(HyprlandState::new(
            std::collections::BTreeMap::new(),
            std::collections::BTreeMap::new(),
            Some(crate::features::workspaces::domain::MonitorName::new("1")),
        ))
        .unwrap();
    let mut scales = hub.monitor_scales_rx().borrow().clone();
    scales.insert(MonitorId::new("1"), Scale::new(1.0));
    hub.monitor_scales_tx().send(scales).unwrap();
}

/// A display server whose first two waits wake the loop and whose third fails, ending it.
fn display_waking_twice() -> FakeDisplayServer {
    let mut call_count: u32 = 0;
    FakeDisplayServer {
        wait_for_events: Box::new(move || {
            call_count = call_count.saturating_add(1);
            if call_count <= 2 {
                Box::pin(std::future::ready(Ok(())))
            } else {
                let exit = DisplayServerError::Internal("Exit".into());
                Box::pin(std::future::ready(Err(exit)))
            }
        }),
        renders: Arc::new(AtomicUsize::new(0)),
    }
}

#[test]
fn test_app_run_commands_and_signals() {
    crate::test_utils::block_on(async {
        let mut harness = harness(Config::default(), FakeRegistry::default());
        queue_one_of_each_command(&harness.queues).await;
        change_every_signal(&harness.hub);

        let bus = bus_with(&harness.hub, FakeSni);
        let result = harness.app.run(display_waking_twice(), bus).await;
        assert!(result.is_err());
    });
}

/// The read model starts from the configuration the app is built with.
#[test]
fn test_new_app_reads_the_configuration_it_starts_with() {
    crate::test_utils::block_on(async {
        let (app, _hub) = app_over(config_sampling_every_2500_ms(), FakeRegistry::default());

        assert_eq!(
            app.read_model.config.metrics().update_interval_ms().value(),
            2500
        );
    });
}
