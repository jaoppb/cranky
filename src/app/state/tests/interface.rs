//! How the app carries out the UI commands module actors send.

use super::fakes::RecordingSni;
use super::fixtures::{drain, harness, systray_action};
use crate::app::test_fakes::FakeRegistry;
use crate::features::module_runtime::test_support::capture_logs;
use crate::features::vdom::domain::UiCommand;
use crate::shared::config::domain::Config;
use std::time::Duration;

/// An `Exec` command runs its text through `sh -c`, and is logged first.
#[test]
fn test_handle_ui_commands_runs_exec_through_the_shell() {
    let marker =
        std::env::temp_dir().join(format!("cranky-test-state-exec-{}", std::process::id()));
    let _ = std::fs::remove_file(&marker);
    let command = format!("printf ran > '{}'", marker.display());
    let logs = capture_logs(|| {
        crate::test_utils::block_on(async {
            let mut harness = harness(Config::default(), FakeRegistry::default());
            harness
                .app
                .handle_ui_commands(UiCommand::Exec(command.clone()), &RecordingSni::new())
                .await;
            for _ in 0..250 {
                if std::fs::read_to_string(&marker).is_ok_and(|text| text == "ran") {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        });
    });

    let ran = std::fs::read_to_string(&marker);
    let _ = std::fs::remove_file(&marker);
    assert_eq!(ran.unwrap(), "ran");
    assert!(logs.contains(&format!(
        "DEBUG cranky::app::state: Executing shell command: {command}"
    )));
}

/// Every command already queued behind the first one is carried out, in the order it was sent.
#[test]
fn test_handle_ui_commands_drains_the_queue_in_order() {
    crate::test_utils::block_on(async {
        let mut harness = harness(Config::default(), FakeRegistry::default());
        let sni = RecordingSni::new();
        harness
            .ui_tx
            .send(systray_action("nm", "Activate"))
            .await
            .unwrap();
        harness
            .ui_tx
            .send(systray_action("vol", "ContextMenu"))
            .await
            .unwrap();

        harness
            .app
            .handle_ui_commands(systray_action("bt", "SecondaryActivate"), &sni)
            .await;

        assert_eq!(
            drain(&sni.calls()),
            [
                "bt SystrayActionCall { name: SecondaryActivate, position: None }",
                "nm SystrayActionCall { name: Activate, position: None }",
                "vol SystrayActionCall { name: ContextMenu, position: None }",
            ]
        );
    });
}

/// A systray action is logged when received and when it succeeds.
#[test]
fn test_handle_ui_commands_logs_a_systray_action_and_its_success() {
    let logs = capture_logs(|| {
        crate::test_utils::block_on(async {
            let mut harness = harness(Config::default(), FakeRegistry::default());
            harness
                .app
                .handle_ui_commands(systray_action("nm", "Activate"), &RecordingSni::new())
                .await;
        });
    });

    assert_eq!(
        logs.lines().collect::<Vec<_>>(),
        [
            "DEBUG cranky::app::state: Received UiCommand::SystrayAction, triggering SNI action id=SystrayId(\"nm\") action=Activate pos=None",
            "DEBUG cranky::app::state: SNI trigger_action succeeded id=SystrayId(\"nm\") action=Activate",
        ]
    );
}

/// A systray action the host refuses is logged as an error, and the commands queued behind it
/// are still carried out.
#[test]
fn test_handle_ui_commands_logs_a_refused_systray_action_and_goes_on() {
    let mut calls = Vec::new();
    let logs = capture_logs(|| {
        crate::test_utils::block_on(async {
            let mut harness = harness(Config::default(), FakeRegistry::default());
            let sni = RecordingSni::failing();
            harness
                .ui_tx
                .send(systray_action("vol", "ContextMenu"))
                .await
                .unwrap();
            harness
                .app
                .handle_ui_commands(systray_action("nm", "Activate"), &sni)
                .await;
            calls = drain(&sni.calls());
        });
    });

    assert_eq!(calls.len(), 2);
    let errors: Vec<_> = logs
        .lines()
        .filter(|line| line.starts_with("ERROR"))
        .collect();
    assert_eq!(
        errors,
        [
            "ERROR cranky::app::state: SNI trigger_action failed id=SystrayId(\"nm\") action=Activate err=ActionFailed { id: \"nm\", error: \"item gone\" }",
            "ERROR cranky::app::state: SNI trigger_action failed id=SystrayId(\"vol\") action=ContextMenu err=ActionFailed { id: \"vol\", error: \"item gone\" }",
        ]
    );
    assert!(!logs.contains("succeeded"));
}
