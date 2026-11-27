//! How the Hyprland adapter fails a query and retries its event socket.
use super::adapter::FakeHyprland;
use crate::features::workspaces::adapters::hyprland_adapter::HyprlandAdapter;
use crate::features::workspaces::adapters::hyprland_provider::{HyprError, HyprlandProvider};
use crate::features::workspaces::adapters::signal_loop::RECONNECT_PAUSE;
use crate::features::workspaces::ports::WindowManagerPort;
use crate::shared::events::signals::SignalHub;
use std::os::unix::net::UnixStream;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

/// The tracing target of the adapter's logs.
const TARGET: &str = "cranky::features::workspaces::adapters::hyprland_adapter";

/// The IPC error `get_state` returns for `provider`, with the requests it
/// made and the logs it wrote.
fn failed_state(provider: FakeHyprland) -> (String, (usize, usize, usize), String) {
    let provider = Arc::new(provider);
    let adapter = HyprlandAdapter::with_provider(provider.clone());
    let mut result = None;
    let logs = crate::features::module_runtime::test_support::capture_logs(|| {
        result = Some(adapter.get_state());
    });
    let Some(Err(error)) = result else {
        panic!("get_state must fail");
    };
    (error.to_string(), provider.requests(), logs)
}

/// A failed workspaces query fails the state before monitors are asked for,
/// and the error is logged inside the `get_state` span.
#[test]
fn test_hyprland_adapter_get_state_fails_on_workspaces() {
    let reason = "Window manager IPC error: Failed to get workspaces: \
                  Hyprland instance signature not found. Is Hyprland running?";
    assert_eq!(
        failed_state(FakeHyprland::answering(None, Some("[]"))),
        (
            reason.to_string(),
            (1, 0, 0),
            format!("ERROR get_state: {TARGET}: error={reason}\n")
        )
    );
}

#[test]
fn test_hyprland_adapter_get_state_fails_on_monitors() {
    let (error, requests, _) = failed_state(FakeHyprland::answering(Some("[]"), None));
    assert_eq!(
        (error.as_str(), requests),
        (
            "Window manager IPC error: Failed to get monitors: \
             Hyprland instance signature not found. Is Hyprland running?",
            (1, 1, 0)
        )
    );
}

/// Unparseable JSON fails the state with the parser's message, workspaces
/// and monitors alike; both are queried before either is parsed.
#[test]
fn test_hyprland_adapter_get_state_fails_on_invalid_json() {
    let parse_error = serde_json::from_str::<Vec<u8>>("nope")
        .unwrap_err()
        .to_string();
    let expected = (
        format!("Window manager IPC error: {parse_error}"),
        (1, 1, 0),
    );
    let (error, requests, _) = failed_state(FakeHyprland::answering(Some("nope"), Some("[]")));
    assert_eq!((error, requests), expected);
    let (error, requests, _) = failed_state(FakeHyprland::answering(Some("[]"), Some("nope")));
    assert_eq!((error, requests), expected);
}

/// Workspaces are parsed before monitors: when both replies are invalid, the
/// workspaces' parse error is the one reported.
#[test]
fn test_hyprland_adapter_get_state_parses_workspaces_first() {
    let parse_error = serde_json::from_str::<Vec<u8>>("nope")
        .unwrap_err()
        .to_string();
    let (error, _, _) = failed_state(FakeHyprland::answering(Some("nope"), Some("[1]")));
    assert_eq!(error, format!("Window manager IPC error: {parse_error}"));
}

/// When several monitors claim focus, the last one in Hyprland's reply wins.
#[test]
fn test_hyprland_adapter_get_state_takes_the_last_focused_monitor() {
    let provider = Arc::new(FakeHyprland::new(
        "[]",
        r#"[{"name": "HDMI-A-1", "activeWorkspace": {"id": 1}, "specialWorkspace": {"id": 0}, "focused": true},
            {"name": "DP-1", "activeWorkspace": {"id": 2}, "specialWorkspace": {"id": 0}, "focused": true}]"#,
    ));
    let (_, _, focused) = HyprlandAdapter::with_provider(provider)
        .get_state()
        .unwrap();
    assert_eq!(
        focused,
        Some(crate::features::workspaces::domain::MonitorName::new(
            "DP-1"
        ))
    );
}

/// A Hyprland whose event socket cannot be opened the first time, and that
/// ends the adapter's reconnect loop by panicking on the second attempt.
#[derive(Default)]
struct RefusingHyprland {
    event_listens: AtomicUsize,
}

impl HyprlandProvider for RefusingHyprland {
    fn query_monitors(&self) -> Result<String, HyprError> {
        Ok("[]".to_string())
    }

    fn query_workspaces(&self) -> Result<String, HyprError> {
        Ok("[]".to_string())
    }

    fn listen_events(&self) -> Result<UnixStream, HyprError> {
        end_on_a_retry(self.event_listens.fetch_add(1, Ordering::SeqCst));
        Err(HyprError::NoInstance)
    }
}

/// Panics on any `attempt` after the first, the only way to end the
/// adapter's reconnect thread.
fn end_on_a_retry(attempt: usize) {
    assert_eq!(attempt, 0, "the adapter retried the event socket");
}

/// A failed connection to the event socket is retried after a one-second pause.
#[test]
fn test_hyprland_adapter_run_retries_a_refused_event_socket() {
    let provider = Arc::new(RefusingHyprland::default());
    let adapter = HyprlandAdapter::with_provider(provider.clone());
    let hub = Arc::new(SignalHub::new(
        crate::shared::config::domain::Config::default(),
    ));
    let started = std::time::Instant::now();
    // The runtime waits for the reconnect thread, which ends on the retry.
    crate::test_utils::block_on(adapter.run(hub));
    assert!(started.elapsed() >= RECONNECT_PAUSE);
    // Pins the pause itself: the elapsed check alone would also pass for a zero pause.
    assert_eq!(RECONNECT_PAUSE, std::time::Duration::from_secs(1));
    assert_eq!(provider.event_listens.load(Ordering::SeqCst), 2);
}
