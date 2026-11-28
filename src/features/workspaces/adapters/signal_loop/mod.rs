//! Follows Hyprland's event socket on a blocking thread: each batch of lines is applied to the
//! state, a full resync runs when the batch asks for one or leaves the state inconsistent, and
//! a changed state is published to the hub.
//!
//! Every log of the loop is written from this module, so they all keep its target.
mod batch;
mod resync;
mod stream;

use super::hyprland_provider::{HyprError, HyprlandProvider};
use super::inconsistency::StateInconsistency;
use crate::features::workspaces::ports::{WindowManagerError, WindowManagerPort};
use crate::shared::events::signals::{HyprlandState, SignalHub};
use std::sync::Arc;
use std::time::Duration;
use stream::process_event_stream;
use tokio::sync::watch;

/// Outcome of processing one raw event line, ordered from the least to the
/// most a batch must do afterwards, so a batch's outcome is its lines' maximum.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum LineOutcome {
    /// The line did not parse into an event.
    Ignored,
    /// The event was applied to state.
    Applied,
    /// Applied, but too thin to apply incrementally: a full resync must run regardless.
    AppliedNeedsResync,
}

/// The raw lines one batch read, kept to log why a resync ran, and the batch's outcome, which
/// decides whether a resync must run regardless of the state.
#[derive(Clone, PartialEq, Eq, Debug)]
struct EventBatch {
    events: Vec<String>,
    outcome: LineOutcome,
}

/// How long the loop waits after the event socket ends or refuses a connection before it
/// connects again.
pub(super) const RECONNECT_PAUSE: Duration = Duration::from_secs(1);

pub(super) async fn run_event_loop<P: WindowManagerPort + 'static>(
    adapter: P,
    provider: Arc<dyn HyprlandProvider>,
    hub: Arc<SignalHub>,
) {
    tokio::task::spawn_blocking(move || {
        let hypr_tx = hub.hyprland_tx();
        loop {
            follow_event_socket(provider.as_ref(), &adapter, &hypr_tx);
            std::thread::sleep(RECONNECT_PAUSE);
        }
    });
}

/// Connects to the event socket and follows it until it ends, or logs why it could not connect.
fn follow_event_socket<P: WindowManagerPort>(
    provider: &dyn HyprlandProvider,
    adapter: &P,
    hypr_tx: &watch::Sender<HyprlandState>,
) {
    match provider.listen_events() {
        Ok(stream) => process_event_stream(stream, adapter, hypr_tx),
        Err(error) => log_connect_failed(&error),
    }
}

/// Logs one raw event line as it arrives.
fn log_event_received(event: &str) {
    tracing::trace!(event, "Hyprland event received");
}

/// Logs a resync forced by a monitor added, on a state with no inconsistency.
fn log_forced_resync(batch_events: &[String]) {
    tracing::debug!(
        batch_events = ?batch_events,
        "Monitor added with no usable payload, forcing full resync"
    );
}

/// Logs a resync forced by the inconsistencies a batch left.
fn log_inconsistent_state(inconsistencies: &[StateInconsistency], batch_events: &[String]) {
    tracing::warn!(
        reasons = ?inconsistencies,
        batch_events = ?batch_events,
        "Hyprland state inconsistent after event batch, forcing full resync"
    );
}

/// Logs a resync whose state query failed.
fn log_resync_failed(error: &WindowManagerError) {
    tracing::error!("Failed to resync Hyprland state after inconsistency: {error}");
}

/// Logs an initial state query that failed.
fn log_initial_fetch_failed(error: &WindowManagerError) {
    tracing::error!("Hyprland adapter error on initial fetch: {error}");
}

/// Logs the event socket closing.
fn log_socket_closed() {
    tracing::info!("Hyprland event socket closed, reconnecting...");
}

/// Logs a failed read on the event socket.
fn log_read_failed(error: &std::io::Error) {
    tracing::error!("Hyprland event socket read error: {error}");
}

/// Logs a failed connection to the event socket.
fn log_connect_failed(error: &HyprError) {
    tracing::error!("Failed to connect to Hyprland event socket: {error}");
}

#[cfg(test)]
mod tests {
    use super::batch::drain_batch;
    use super::resync::{check_and_resync, fetch_initial_state};
    use super::*;
    use crate::features::workspaces::adapters::tests::fixtures::{self, monitor, workspace};
    use crate::features::workspaces::domain::{
        Monitor, MonitorName, Workspace, WorkspaceId, WorkspaceName,
    };
    use crate::features::workspaces::ports::{WindowManagerError, WindowManagerState};
    use std::collections::BTreeMap;
    use std::io::{Cursor, ErrorKind};
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// A window manager that reports `state`, or an IPC error when it has
    /// none, and counts how often it was asked.
    struct FakeWindowManager {
        state: Option<WindowManagerState>,
        queries: AtomicUsize,
    }

    impl FakeWindowManager {
        const fn new(state: Option<WindowManagerState>) -> Self {
            Self {
                state,
                queries: AtomicUsize::new(0),
            }
        }

        fn queries(&self) -> usize {
            self.queries.load(Ordering::SeqCst)
        }
    }

    impl WindowManagerPort for FakeWindowManager {
        fn get_state(&self) -> Result<WindowManagerState, WindowManagerError> {
            self.queries.fetch_add(1, Ordering::SeqCst);
            self.state
                .clone()
                .ok_or_else(|| WindowManagerError::IpcError {
                    reason: "boom".to_string(),
                })
        }
    }

    #[test]
    fn test_drain_batch_coalesces_multiple_lines() {
        let mut state = HyprlandState::new(BTreeMap::new(), BTreeMap::new(), None);
        let mut line_buf = "workspacev2>>1,test_ws\n".to_string();
        let mut reader = Cursor::new(b"focusedmonv2>>DP-1,1\ninvalid>>data\n".to_vec());

        let batch = drain_batch(&mut reader, &mut line_buf, &mut state);

        assert_eq!(batch.outcome, LineOutcome::Applied);
        assert_eq!(
            batch.events,
            vec![
                "workspacev2>>1,test_ws".to_string(),
                "focusedmonv2>>DP-1,1".to_string(),
                "invalid>>data".to_string(),
            ]
        );
        assert!(state.workspaces().contains_key(&WorkspaceId::new(1)));
        assert_eq!(state.focused_monitor(), Some(&MonitorName::new("DP-1")));
        assert!(state.monitors().contains_key(&MonitorName::new("DP-1")));
    }

    #[test]
    fn test_drain_batch_unparseable_line_does_not_mark_state_changed() {
        let mut state = HyprlandState::new(BTreeMap::new(), BTreeMap::new(), None);
        let mut line_buf = "invalid>>data\n".to_string();
        let mut reader = Cursor::new(Vec::new());

        let batch = drain_batch(&mut reader, &mut line_buf, &mut state);

        assert_eq!(batch.outcome, LineOutcome::Ignored);
        assert_eq!(batch.events, vec!["invalid>>data".to_string()]);
    }

    #[test]
    fn test_drain_batch_applies_monitor_removed() {
        let mut monitors = BTreeMap::new();
        monitors.insert(
            MonitorName::new("HDMI-A-1"),
            Monitor::new(MonitorName::new("HDMI-A-1"), WorkspaceId::new(4), None),
        );
        let mut state = HyprlandState::new(BTreeMap::new(), monitors, None);
        let mut line_buf =
            "monitorremovedv2>>1,HDMI-A-1,LG Electronics LG FULL HD 206AZQV5S860\n".to_string();
        let mut reader = Cursor::new(Vec::new());

        let batch = drain_batch(&mut reader, &mut line_buf, &mut state);

        assert_eq!(batch.outcome, LineOutcome::Applied);
        assert!(!state.monitors().contains_key(&MonitorName::new("HDMI-A-1")));
    }

    #[test]
    fn test_drain_batch_monitor_added_forces_resync() {
        let mut state = HyprlandState::new(BTreeMap::new(), BTreeMap::new(), None);
        let mut line_buf = "monitoraddedv2>>1,HDMI-A-1,LG Display\n".to_string();
        let mut reader = Cursor::new(Vec::new());

        let batch = drain_batch(&mut reader, &mut line_buf, &mut state);

        assert_eq!(batch.outcome, LineOutcome::AppliedNeedsResync);
        // No usable payload - applying it is a no-op; the resync is what fixes state up.
        assert!(state.monitors().is_empty());
    }

    /// A batch whose first line is ignored still changes state and forces a
    /// resync when a later line is a monitor added.
    #[test]
    fn test_drain_batch_takes_the_outcome_of_a_later_line() {
        let mut state = HyprlandState::new(BTreeMap::new(), BTreeMap::new(), None);
        let mut line_buf = "invalid>>data\n".to_string();
        let mut reader = Cursor::new(b"monitoraddedv2>>1,HDMI-A-1,x\n".to_vec());

        let batch = drain_batch(&mut reader, &mut line_buf, &mut state);

        assert_eq!(batch.outcome, LineOutcome::AppliedNeedsResync);
    }

    /// An applied first line does not mask the forced resync a later
    /// monitor added asks for.
    #[test]
    fn test_drain_batch_forces_resync_for_a_later_monitor_added() {
        let mut state = HyprlandState::new(BTreeMap::new(), BTreeMap::new(), None);
        let mut line_buf = "workspacev2>>1,a\n".to_string();
        let mut reader = Cursor::new(b"monitoraddedv2>>1,HDMI-A-1,x\n".to_vec());

        let batch = drain_batch(&mut reader, &mut line_buf, &mut state);

        assert_eq!(batch.outcome, LineOutcome::AppliedNeedsResync);
    }

    /// A batch of `events` that applied without asking for a forced resync.
    fn applied(events: &[&str]) -> EventBatch {
        EventBatch {
            events: events.iter().map(ToString::to_string).collect(),
            outcome: LineOutcome::Applied,
        }
    }

    fn inconsistent_state() -> HyprlandState {
        let mut workspaces = BTreeMap::new();
        workspaces.insert(
            WorkspaceId::new(4),
            crate::features::workspaces::domain::Workspace::new(
                WorkspaceId::new(4),
                WorkspaceName::new("4"),
                Some(MonitorName::new("HDMI-A-1")),
            ),
        );
        HyprlandState::new(workspaces, BTreeMap::new(), None)
    }

    /// What a window manager whose state is `state` reports.
    fn reported(state: &HyprlandState) -> WindowManagerState {
        (
            state.workspaces().clone(),
            state.monitors().clone(),
            state.focused_monitor().cloned(),
        )
    }

    #[test]
    fn test_check_and_resync_replaces_state_when_inconsistent() {
        let mut state = inconsistent_state();
        let mock = FakeWindowManager::new(Some(reported(&fixtures::state(
            vec![workspace(4, "4", Some("eDP-1"))],
            vec![monitor("eDP-1", 4, None)],
            Some("eDP-1"),
        ))));

        check_and_resync(&mock, &mut state, &applied(&["monitorremoved>>HDMI-A-1"]));

        let workspace_4 = state.workspaces().get(&WorkspaceId::new(4));
        assert_eq!(
            (
                mock.queries(),
                workspace_4.and_then(Workspace::monitor),
                state.monitors().keys().collect::<Vec<_>>()
            ),
            (
                1,
                Some(&MonitorName::new("eDP-1")),
                vec![&MonitorName::new("eDP-1")]
            )
        );
    }

    #[test]
    fn test_check_and_resync_leaves_consistent_state_untouched() {
        let mut workspaces = BTreeMap::new();
        workspaces.insert(
            WorkspaceId::new(1),
            crate::features::workspaces::domain::Workspace::new(
                WorkspaceId::new(1),
                WorkspaceName::new("1"),
                Some(MonitorName::new("DP-1")),
            ),
        );
        let mut monitors = BTreeMap::new();
        monitors.insert(
            MonitorName::new("DP-1"),
            Monitor::new(MonitorName::new("DP-1"), WorkspaceId::new(1), None),
        );
        let mut state = HyprlandState::new(workspaces, monitors, Some(MonitorName::new("DP-1")));
        let before = state.clone();

        let mock = FakeWindowManager::new(None);

        check_and_resync(&mock, &mut state, &applied(&[]));

        assert_eq!(mock.queries(), 0);
        assert_eq!(state, before);
    }

    #[test]
    fn test_check_and_resync_forces_resync_on_consistent_state_when_flagged() {
        let mut state = fixtures::state(
            vec![workspace(1, "1", Some("DP-1"))],
            vec![monitor("DP-1", 1, None)],
            Some("DP-1"),
        );
        let new_monitors = fixtures::state(vec![], vec![monitor("HDMI-A-1", 2, None)], None);
        let mock = FakeWindowManager::new(Some(reported(&new_monitors)));

        let batch = EventBatch {
            events: vec!["monitoraddedv2>>1,HDMI-A-1,x".to_string()],
            outcome: LineOutcome::AppliedNeedsResync,
        };
        check_and_resync(&mock, &mut state, &batch);

        assert_eq!(
            (mock.queries(), state.monitors().keys().collect::<Vec<_>>()),
            (1, vec![&MonitorName::new("HDMI-A-1")])
        );
    }

    /// Runs one event stream to its end with `lines` already written, and
    /// returns how often the window manager was queried and the state the
    /// hub was left with.
    fn stream(lines: &[u8], reported: WindowManagerState) -> (usize, HyprlandState) {
        use std::io::Write;

        let (mut sender, receiver) = std::os::unix::net::UnixStream::pair().unwrap();
        sender.write_all(lines).unwrap();
        drop(sender);
        let fake = FakeWindowManager::new(Some(reported));
        let (hypr_tx, hypr_rx) =
            tokio::sync::watch::channel(HyprlandState::new(BTreeMap::new(), BTreeMap::new(), None));
        process_event_stream(receiver, &fake, &hypr_tx);
        let state = hypr_rx.borrow().clone();
        (fake.queries(), state)
    }

    fn one_monitor(name: &str) -> WindowManagerState {
        let mut monitors = BTreeMap::new();
        monitors.insert(
            MonitorName::new(name),
            Monitor::new(MonitorName::new(name), WorkspaceId::new(1), None),
        );
        (BTreeMap::new(), monitors, None)
    }

    /// One monitor whose active workspace exists and sits on it, so the state has no
    /// inconsistency and only a forced resync refetches it.
    fn consistent_monitor(name: &str) -> WindowManagerState {
        let (mut workspaces, monitors, active) = one_monitor(name);
        workspaces.insert(
            WorkspaceId::new(1),
            crate::features::workspaces::domain::Workspace::new(
                WorkspaceId::new(1),
                WorkspaceName::new("1"),
                Some(MonitorName::new(name)),
            ),
        );
        (workspaces, monitors, active)
    }

    #[test]
    fn test_process_event_stream_resyncs_after_a_monitor_added() {
        let (queries, state) = stream(
            b"monitoraddedv2>>1,HDMI-A-1,x\n",
            consistent_monitor("HDMI-A-1"),
        );
        // The initial fetch, then the forced resync.
        assert_eq!(queries, 2);
        assert!(state.monitors().contains_key(&MonitorName::new("HDMI-A-1")));
    }

    #[test]
    fn test_process_event_stream_applies_a_consistent_event_without_resync() {
        let (queries, state) = stream(b"monitorremovedv2>>1,DP-1,x\n", one_monitor("DP-1"));
        assert_eq!(queries, 1);
        assert!(state.monitors().is_empty());
    }

    #[test]
    fn test_process_event_stream_ignores_an_unparseable_line() {
        let (queries, state) = stream(b"invalid>>data\n", one_monitor("DP-1"));
        assert_eq!(queries, 1);
        assert!(state.monitors().contains_key(&MonitorName::new("DP-1")));
    }

    #[test]
    fn test_check_and_resync_leaves_state_as_is_on_get_state_error() {
        let mut state = inconsistent_state();
        let before = state.clone();

        let mock = FakeWindowManager::new(None);

        check_and_resync(&mock, &mut state, &applied(&[]));

        assert_eq!(mock.queries(), 1);
        assert_eq!(state, before);
    }

    /// The tracing target of every log this loop writes.
    const TARGET: &str = "cranky::features::workspaces::adapters::signal_loop";

    /// A reader that hands out `chunks` one read at a time, then the end of the stream.
    struct ScriptedReader {
        chunks: std::collections::VecDeque<std::io::Result<&'static [u8]>>,
    }

    impl std::io::Read for ScriptedReader {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            let bytes = self.chunks.pop_front().unwrap_or(Ok(b""))?;
            buf[..bytes.len()].copy_from_slice(bytes);
            Ok(bytes.len())
        }
    }

    /// Drains one batch that starts with `workspacev2>>1,a` from a reader that
    /// answers `chunks`, and returns the batch and the state it left.
    fn drain_scripted(chunks: Vec<std::io::Result<&'static [u8]>>) -> (EventBatch, HyprlandState) {
        let mut state = HyprlandState::new(BTreeMap::new(), BTreeMap::new(), None);
        let mut line_buf = "workspacev2>>1,a\n".to_string();
        let mut reader = std::io::BufReader::new(ScriptedReader {
            chunks: chunks.into(),
        });
        let batch = drain_batch(&mut reader, &mut line_buf, &mut state);
        (batch, state)
    }

    #[test]
    fn test_drain_batch_leaves_blank_lines_out_of_the_events() {
        let (batch, _) = drain_scripted(vec![Ok(b"\n   \nfocusedmonv2>>DP-1,1\n")]);
        assert_eq!(
            (batch.events, batch.outcome),
            (
                vec![
                    "workspacev2>>1,a".to_string(),
                    "focusedmonv2>>DP-1,1".to_string()
                ],
                LineOutcome::Applied
            )
        );
    }

    #[test]
    fn test_drain_batch_stops_at_a_timeout() {
        let timeout = std::io::Error::from(ErrorKind::TimedOut);
        let (batch, state) = drain_scripted(vec![
            Ok(b"workspacev2>>2,b\n"),
            Err(timeout),
            Ok(b"workspacev2>>3,c\n"),
        ]);
        assert_eq!(batch.events.len(), 2);
        assert!(!state.workspaces().contains_key(&WorkspaceId::new(3)));
    }

    #[test]
    fn test_drain_batch_stops_when_the_read_would_block() {
        let would_block = std::io::Error::from(ErrorKind::WouldBlock);
        let (batch, state) = drain_scripted(vec![Err(would_block), Ok(b"workspacev2>>3,c\n")]);
        assert_eq!(batch.events, vec!["workspacev2>>1,a".to_string()]);
        assert!(!state.workspaces().contains_key(&WorkspaceId::new(3)));
    }

    /// Any other read error ends the batch too: here a line that is not UTF-8.
    #[test]
    fn test_drain_batch_stops_at_a_read_error() {
        let (batch, state) = drain_scripted(vec![
            Ok(b"workspacev2>>2,b\n\xff\n"),
            Ok(b"workspacev2>>3,c\n"),
        ]);
        assert_eq!(batch.events.len(), 2);
        assert!(state.workspaces().contains_key(&WorkspaceId::new(2)));
        assert!(!state.workspaces().contains_key(&WorkspaceId::new(3)));
    }

    #[test]
    fn test_drain_batch_traces_every_non_blank_line() {
        let logs = crate::features::module_runtime::test_support::capture_logs(|| {
            drain_scripted(vec![Ok(b"\ninvalid>>data\n")]);
        });
        assert_eq!(
            logs,
            format!(
                "TRACE {TARGET}: Hyprland event received event=\"workspacev2>>1,a\"\n\
                 TRACE {TARGET}: Hyprland event received event=\"invalid>>data\"\n"
            )
        );
    }

    /// Runs `check_and_resync` on `state` and `batch` against a window
    /// manager reporting `reported`, and returns the logs it wrote.
    fn resync_logs(
        mut state: HyprlandState,
        batch: &EventBatch,
        reported: Option<WindowManagerState>,
    ) -> String {
        let fake = FakeWindowManager::new(reported);
        crate::features::module_runtime::test_support::capture_logs(|| {
            check_and_resync(&fake, &mut state, batch);
        })
    }

    #[test]
    fn test_check_and_resync_logs_a_forced_resync_at_debug() {
        let batch = EventBatch {
            events: vec!["monitoraddedv2>>1,DP-1".to_string()],
            outcome: LineOutcome::AppliedNeedsResync,
        };
        let state = HyprlandState::new(BTreeMap::new(), BTreeMap::new(), None);
        let logs = resync_logs(state, &batch, Some(consistent_monitor("DP-1")));
        assert_eq!(
            logs,
            format!(
                "DEBUG {TARGET}: Monitor added with no usable payload, forcing full resync \
                 batch_events=[\"monitoraddedv2>>1,DP-1\"]\n"
            )
        );
    }

    /// An inconsistent state logs the warning alone, even when the batch also forced a resync.
    #[test]
    fn test_check_and_resync_warns_about_an_inconsistent_state() {
        let batch = EventBatch {
            events: vec!["e".to_string()],
            outcome: LineOutcome::AppliedNeedsResync,
        };
        let logs = resync_logs(
            inconsistent_state(),
            &batch,
            Some(consistent_monitor("DP-1")),
        );
        assert_eq!(
            logs,
            format!(
                " WARN {TARGET}: Hyprland state inconsistent after event batch, forcing full resync \
                 reasons=[WorkspaceMonitorNotFound {{ workspace_id: WorkspaceId(4), \
                 workspace_name: WorkspaceName(\"4\"), monitor_name: MonitorName(\"HDMI-A-1\") }}] \
                 batch_events=[\"e\"]\n"
            )
        );
    }

    #[test]
    fn test_check_and_resync_logs_a_failed_resync() {
        let logs = resync_logs(inconsistent_state(), &applied(&[]), None);
        assert!(logs.ends_with(&format!(
            "ERROR {TARGET}: Failed to resync Hyprland state after inconsistency: \
             Window manager IPC error: boom\n"
        )));
    }

    #[test]
    fn test_fetch_initial_state_falls_back_to_an_empty_state_on_error() {
        let empty = HyprlandState::new(BTreeMap::new(), BTreeMap::new(), None);
        let (hypr_tx, mut hypr_rx) = tokio::sync::watch::channel(inconsistent_state());
        hypr_rx.borrow_and_update();
        let mut state = inconsistent_state();
        let logs = crate::features::module_runtime::test_support::capture_logs(|| {
            state = fetch_initial_state(&FakeWindowManager::new(None), &hypr_tx);
        });
        assert_eq!(state, empty);
        assert!(!hypr_rx.has_changed().unwrap());
        assert_eq!(
            logs,
            format!(
                "ERROR {TARGET}: Hyprland adapter error on initial fetch: \
                 Window manager IPC error: boom\n"
            )
        );
    }

    #[test]
    fn test_fetch_initial_state_publishes_a_changed_state() {
        let (hypr_tx, mut hypr_rx) = tokio::sync::watch::channel(inconsistent_state());
        hypr_rx.borrow_and_update();
        let fake = FakeWindowManager::new(Some(consistent_monitor("DP-1")));
        let state = fetch_initial_state(&fake, &hypr_tx);
        let (workspaces, monitors, focused) = consistent_monitor("DP-1");
        let expected = HyprlandState::new(workspaces, monitors, focused);
        assert_eq!(state, expected);
        assert!(hypr_rx.has_changed().unwrap());
        assert_eq!(*hypr_rx.borrow(), expected);
    }

    /// Runs one event stream of `lines` against a hub that already holds the
    /// state the window manager reports, and tells whether the hub was notified.
    fn stream_notifies(lines: &[u8], reported: &WindowManagerState) -> bool {
        use std::io::Write;

        let (mut sender, receiver) = std::os::unix::net::UnixStream::pair().unwrap();
        sender.write_all(lines).unwrap();
        drop(sender);
        let (workspaces, monitors, focused) = reported.clone();
        let (hypr_tx, mut hypr_rx) =
            tokio::sync::watch::channel(HyprlandState::new(workspaces, monitors, focused));
        hypr_rx.borrow_and_update();
        let fake = FakeWindowManager::new(Some(reported.clone()));
        process_event_stream(receiver, &fake, &hypr_tx);
        hypr_rx.has_changed().unwrap()
    }

    #[test]
    fn test_process_event_stream_does_not_notify_an_unchanged_state() {
        let reported = consistent_monitor("DP-1");
        assert!(!stream_notifies(b"invalid>>data\n", &reported));
        assert!(!stream_notifies(b"monitoraddedv2>>1,DP-1\n", &reported));
        assert!(stream_notifies(b"renameworkspace>>1,one\n", &reported));
    }

    #[test]
    fn test_process_event_stream_logs_a_closed_socket() {
        let logs = crate::features::module_runtime::test_support::capture_logs(|| {
            stream(b"", one_monitor("DP-1"));
        });
        assert_eq!(
            logs,
            format!(" INFO {TARGET}: Hyprland event socket closed, reconnecting...\n")
        );
    }

    /// A Hyprland whose event socket refuses the first connection, then hands out a socket
    /// that has already ended.
    #[derive(Default)]
    struct RefusingOnceHyprland {
        listens: AtomicUsize,
    }

    impl HyprlandProvider for RefusingOnceHyprland {
        fn query_monitors(&self) -> Result<String, HyprError> {
            Ok("[]".to_string())
        }

        fn query_workspaces(&self) -> Result<String, HyprError> {
            Ok("[]".to_string())
        }

        fn listen_events(&self) -> Result<std::os::unix::net::UnixStream, HyprError> {
            match self.listens.fetch_add(1, Ordering::SeqCst) {
                0 => Err(HyprError::InputOutput(ErrorKind::ConnectionRefused.into())),
                _ => ended_socket(),
            }
        }
    }

    /// The reading end of a socket whose writing end is already closed.
    fn ended_socket() -> Result<std::os::unix::net::UnixStream, HyprError> {
        let (sender, receiver) = std::os::unix::net::UnixStream::pair()?;
        drop(sender);
        Ok(receiver)
    }

    /// A refused connection is logged as an error and gives up the attempt without querying
    /// the window manager; the next attempt connects and follows the socket, which queries it.
    #[test]
    fn test_follow_event_socket_logs_a_refused_connection_then_retries() {
        let provider = RefusingOnceHyprland::default();
        let fake = FakeWindowManager::new(Some(one_monitor("DP-1")));
        let (hypr_tx, _hypr_rx) =
            tokio::sync::watch::channel(HyprlandState::new(BTreeMap::new(), BTreeMap::new(), None));
        let logs = crate::features::module_runtime::test_support::capture_logs(|| {
            follow_event_socket(&provider, &fake, &hypr_tx);
        });
        let queries_after_refusal = fake.queries();
        follow_event_socket(&provider, &fake, &hypr_tx);
        assert_eq!(
            (queries_after_refusal, fake.queries(), logs),
            (
                0,
                1,
                format!(
                    "ERROR {TARGET}: Failed to connect to Hyprland event socket: \
                     IO error: connection refused\n"
                )
            )
        );
    }

    #[test]
    fn test_process_event_stream_stops_at_a_read_error() {
        let mut queries = 0;
        let logs = crate::features::module_runtime::test_support::capture_logs(|| {
            (queries, _) = stream(b"\xff\nworkspacev2>>1,a\n", one_monitor("DP-1"));
        });
        assert_eq!(queries, 1);
        assert_eq!(
            logs,
            format!(
                "ERROR {TARGET}: Hyprland event socket read error: \
                 stream did not contain valid UTF-8\n"
            )
        );
    }
}
