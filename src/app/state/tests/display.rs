//! How the app carries out the display commands module actors send.

use super::fakes::RecordingDisplay;
use super::fixtures::{drain, harness, hide_tooltip, show_tooltip};
use crate::app::test_fakes::FakeRegistry;
use crate::features::layout_engine::domain::DisplayCommand;
use crate::features::module_runtime::test_support::capture_logs;
use crate::shared::config::domain::Config;

/// Every command already queued behind the first one is carried out in the order it was sent,
/// and however many of them ask for a render, the display renders once, after all of them.
#[test]
fn test_handle_display_commands_drains_the_queue_and_renders_once_at_the_end() {
    crate::test_utils::block_on(async {
        let mut harness = harness(Config::default(), FakeRegistry::default());
        let mut display = RecordingDisplay::new();
        let calls = display.calls();
        for command in [
            DisplayCommand::RequestRender,
            hide_tooltip(),
            DisplayCommand::RequestRender,
            show_tooltip(),
        ] {
            harness.display_tx.send(command).await.unwrap();
        }

        harness
            .app
            .handle_display_commands(show_tooltip(), &mut display);

        assert_eq!(
            drain(&calls),
            ["show Tooltip", "hide Tooltip", "show Tooltip", "render"]
        );
    });
}

/// Floating-surface requests alone do not render the bar.
#[test]
fn test_handle_display_commands_renders_only_when_asked() {
    crate::test_utils::block_on(async {
        let mut harness = harness(Config::default(), FakeRegistry::default());
        let mut display = RecordingDisplay::new();
        let calls = display.calls();
        harness.display_tx.send(hide_tooltip()).await.unwrap();

        harness
            .app
            .handle_display_commands(show_tooltip(), &mut display);

        assert_eq!(drain(&calls), ["show Tooltip", "hide Tooltip"]);
    });
}

/// Each floating-surface request is logged when received and when it succeeds.
#[test]
fn test_handle_display_commands_logs_each_floating_request_it_carries_out() {
    let logs = capture_logs(|| {
        crate::test_utils::block_on(async {
            let mut harness = harness(Config::default(), FakeRegistry::default());
            harness.display_tx.send(hide_tooltip()).await.unwrap();
            harness
                .app
                .handle_display_commands(show_tooltip(), &mut RecordingDisplay::new());
        });
    });

    assert_eq!(
        logs.lines().collect::<Vec<_>>(),
        [
            "DEBUG cranky::app::state: Received DisplayCommand::ShowFloatingSurface, calling display.show_floating_surface kind=Tooltip monitor_id=None anchor_rect=None offset=None parent=None",
            "DEBUG cranky::app::state: display.show_floating_surface succeeded",
            "DEBUG cranky::app::state: Received DisplayCommand::HideFloatingSurface, calling display.hide_floating_surface kind=Tooltip",
            "DEBUG cranky::app::state: display.hide_floating_surface succeeded",
        ]
    );
}

/// A floating-surface request the display refuses is logged as an error, and the commands
/// queued behind it are still carried out.
#[test]
fn test_handle_display_commands_logs_a_refused_request_and_goes_on() {
    let mut calls = Vec::new();
    let logs = capture_logs(|| {
        crate::test_utils::block_on(async {
            let mut harness = harness(Config::default(), FakeRegistry::default());
            let mut display = RecordingDisplay::failing();
            harness.display_tx.send(hide_tooltip()).await.unwrap();
            harness
                .display_tx
                .send(DisplayCommand::RequestRender)
                .await
                .unwrap();
            harness
                .app
                .handle_display_commands(show_tooltip(), &mut display);
            calls = drain(&display.calls());
        });
    });

    assert_eq!(calls, ["show Tooltip", "hide Tooltip", "render"]);
    let errors: Vec<_> = logs
        .lines()
        .filter(|line| line.starts_with("ERROR"))
        .collect();
    assert_eq!(
        errors,
        [
            "ERROR cranky::app::state: display.show_floating_surface failed err=Internal(\"no compositor\")",
            "ERROR cranky::app::state: display.hide_floating_surface failed err=Internal(\"no compositor\")",
        ]
    );
    assert!(!logs.contains("succeeded"));
}
