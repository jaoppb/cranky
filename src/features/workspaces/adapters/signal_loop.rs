use super::event_parser::parse_event;
use super::hyprland_provider::HyprlandProvider;
use super::inconsistency::find_state_inconsistencies;
use crate::features::workspaces::ports::WindowManagerPort;
use crate::shared::events::core::WindowManagerEvent;
use crate::shared::events::signals::{HyprlandState, SignalHub};
use std::io::{BufRead, ErrorKind};
use std::sync::Arc;

/// Outcome of processing one raw event line.
struct LineOutcome {
    /// Whether the line parsed into an event that was applied to state.
    applied: bool,
    /// Whether this event's data is too thin to apply incrementally and a
    /// full resync must run regardless of `find_state_inconsistencies`.
    force_resync: bool,
}

fn process_line(
    line: &str,
    current_state: &mut HyprlandState,
    batch_events: &mut Vec<String>,
) -> LineOutcome {
    let trimmed = line.trim();
    if !trimmed.is_empty() {
        tracing::trace!(event = trimmed, "Hyprland event received");
        batch_events.push(trimmed.to_string());
    }
    let Some(event) = parse_event(line) else {
        return LineOutcome {
            applied: false,
            force_resync: false,
        };
    };
    let force_resync = matches!(event, WindowManagerEvent::MonitorAdded { .. });
    current_state.apply_event(&event);
    LineOutcome {
        applied: true,
        force_resync,
    }
}

fn drain_batch<R: BufRead>(
    reader: &mut R,
    line_buf: &mut String,
    current_state: &mut HyprlandState,
    batch_events: &mut Vec<String>,
) -> (bool, bool) {
    let first = process_line(line_buf, current_state, batch_events);
    let mut state_changed = first.applied;
    let mut force_resync = first.force_resync;
    loop {
        line_buf.clear();
        match reader.read_line(line_buf) {
            Ok(0) => break,
            Ok(_) => {
                let outcome = process_line(line_buf, current_state, batch_events);
                state_changed = state_changed || outcome.applied;
                force_resync = force_resync || outcome.force_resync;
            }
            Err(e) if e.kind() == ErrorKind::WouldBlock || e.kind() == ErrorKind::TimedOut => {
                break;
            }
            Err(_) => break,
        }
    }
    (state_changed, force_resync)
}

fn check_and_resync<P: WindowManagerPort>(
    adapter: &P,
    current_state: &mut HyprlandState,
    batch_events: &[String],
    force_resync: bool,
) {
    let inconsistencies = find_state_inconsistencies(current_state);
    if force_resync || !inconsistencies.is_empty() {
        if inconsistencies.is_empty() {
            tracing::debug!(
                batch_events = ?batch_events,
                "Monitor added with no usable payload, forcing full resync"
            );
        } else {
            tracing::warn!(
                reasons = ?inconsistencies,
                batch_events = ?batch_events,
                "Hyprland state inconsistent after event batch, forcing full resync"
            );
        }
        match adapter.get_state() {
            Ok((workspaces, monitors, focused)) => {
                *current_state = HyprlandState::new(workspaces, monitors, focused);
            }
            Err(e) => {
                tracing::error!("Failed to resync Hyprland state after inconsistency: {e}");
            }
        }
    }
}

fn fetch_initial_state<P: WindowManagerPort>(
    adapter: &P,
    hypr_tx: &tokio::sync::watch::Sender<HyprlandState>,
) -> HyprlandState {
    match adapter.get_state() {
        Ok((workspaces, monitors, focused)) => {
            let state = HyprlandState::new(workspaces, monitors, focused);
            if *hypr_tx.borrow() != state {
                let _ = hypr_tx.send(state.clone());
            }
            state
        }
        Err(e) => {
            tracing::error!("Hyprland adapter error on initial fetch: {e}");
            HyprlandState::new(
                std::collections::BTreeMap::new(),
                std::collections::BTreeMap::new(),
                None,
            )
        }
    }
}

fn process_event_stream<P: WindowManagerPort>(
    stream: std::os::unix::net::UnixStream,
    adapter: &P,
    hypr_tx: &tokio::sync::watch::Sender<HyprlandState>,
) {
    let _ = stream.set_read_timeout(Some(std::time::Duration::from_millis(2)));
    let mut reader = std::io::BufReader::new(stream);
    let mut current_state = fetch_initial_state(adapter, hypr_tx);

    let mut line = String::new();
    loop {
        line.clear();
        let _ = reader.get_mut().set_read_timeout(None);

        match reader.read_line(&mut line) {
            Ok(0) => {
                tracing::info!("Hyprland event socket closed, reconnecting...");
                break;
            }
            Ok(_) => {
                let mut batch_events = Vec::new();
                let _ = reader
                    .get_mut()
                    .set_read_timeout(Some(std::time::Duration::from_millis(2)));

                let (state_changed, force_resync) = drain_batch(
                    &mut reader,
                    &mut line,
                    &mut current_state,
                    &mut batch_events,
                );

                if state_changed || force_resync {
                    check_and_resync(adapter, &mut current_state, &batch_events, force_resync);
                    if *hypr_tx.borrow() != current_state {
                        let _ = hypr_tx.send(current_state.clone());
                    }
                }
            }
            Err(e) => {
                tracing::error!("Hyprland event socket read error: {e}");
                break;
            }
        }
    }
}

pub async fn run_event_loop<P: WindowManagerPort + 'static>(
    adapter: P,
    provider: Arc<dyn HyprlandProvider>,
    hub: Arc<SignalHub>,
) {
    tokio::task::spawn_blocking(move || {
        let hypr_tx = hub.hyprland_tx();
        loop {
            match provider.listen_events() {
                Ok(stream) => process_event_stream(stream, &adapter, &hypr_tx),
                Err(e) => {
                    tracing::error!("Failed to connect to Hyprland event socket: {e}");
                }
            }
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::workspaces::domain::{Monitor, MonitorName, WorkspaceId, WorkspaceName};
    use crate::features::workspaces::ports::{MockWindowManagerPort, WindowManagerError};
    use std::collections::BTreeMap;
    use std::io::Cursor;

    #[test]
    fn test_drain_batch_coalesces_multiple_lines() {
        let mut state = HyprlandState::new(BTreeMap::new(), BTreeMap::new(), None);
        let mut line_buf = "workspacev2>>1,test_ws\n".to_string();
        let mut reader = Cursor::new(b"focusedmonv2>>DP-1,1\ninvalid>>data\n".to_vec());
        let mut batch_events = Vec::new();

        let (state_changed, force_resync) =
            drain_batch(&mut reader, &mut line_buf, &mut state, &mut batch_events);

        assert!(state_changed);
        assert!(!force_resync);
        assert_eq!(
            batch_events,
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
        let mut batch_events = Vec::new();

        let (state_changed, force_resync) =
            drain_batch(&mut reader, &mut line_buf, &mut state, &mut batch_events);

        assert!(!state_changed);
        assert!(!force_resync);
        assert_eq!(batch_events, vec!["invalid>>data".to_string()]);
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
        let mut batch_events = Vec::new();

        let (state_changed, force_resync) =
            drain_batch(&mut reader, &mut line_buf, &mut state, &mut batch_events);

        assert!(state_changed);
        assert!(!force_resync);
        assert!(!state.monitors().contains_key(&MonitorName::new("HDMI-A-1")));
    }

    #[test]
    fn test_drain_batch_monitor_added_forces_resync() {
        let mut state = HyprlandState::new(BTreeMap::new(), BTreeMap::new(), None);
        let mut line_buf = "monitoraddedv2>>1,HDMI-A-1,LG Display\n".to_string();
        let mut reader = Cursor::new(Vec::new());
        let mut batch_events = Vec::new();

        let (state_changed, force_resync) =
            drain_batch(&mut reader, &mut line_buf, &mut state, &mut batch_events);

        assert!(state_changed);
        assert!(force_resync);
        // No usable payload - applying it is a no-op; the resync is what fixes state up.
        assert!(state.monitors().is_empty());
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

    #[test]
    fn test_check_and_resync_replaces_state_when_inconsistent() {
        let mut state = inconsistent_state();

        let mut new_workspaces = BTreeMap::new();
        new_workspaces.insert(
            WorkspaceId::new(4),
            crate::features::workspaces::domain::Workspace::new(
                WorkspaceId::new(4),
                WorkspaceName::new("4"),
                Some(MonitorName::new("eDP-1")),
            ),
        );
        let mut new_monitors = BTreeMap::new();
        new_monitors.insert(
            MonitorName::new("eDP-1"),
            Monitor::new(MonitorName::new("eDP-1"), WorkspaceId::new(4), None),
        );

        let mut mock = MockWindowManagerPort::new();
        mock.expect_get_state().times(1).returning(move || {
            Ok((
                new_workspaces.clone(),
                new_monitors.clone(),
                Some(MonitorName::new("eDP-1")),
            ))
        });

        check_and_resync(
            &mock,
            &mut state,
            &["monitorremoved>>HDMI-A-1".to_string()],
            false,
        );

        assert!(state.workspaces().contains_key(&WorkspaceId::new(4)));
        assert_eq!(
            state
                .workspaces()
                .get(&WorkspaceId::new(4))
                .unwrap()
                .monitor(),
            Some(&MonitorName::new("eDP-1"))
        );
        assert!(state.monitors().contains_key(&MonitorName::new("eDP-1")));
        assert!(!state.monitors().contains_key(&MonitorName::new("HDMI-A-1")));
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

        let mut mock = MockWindowManagerPort::new();
        mock.expect_get_state().times(0);

        check_and_resync(&mock, &mut state, &[], false);

        assert_eq!(state, before);
    }

    #[test]
    fn test_check_and_resync_forces_resync_on_consistent_state_when_flagged() {
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

        let mut new_monitors = BTreeMap::new();
        new_monitors.insert(
            MonitorName::new("HDMI-A-1"),
            Monitor::new(MonitorName::new("HDMI-A-1"), WorkspaceId::new(2), None),
        );
        let mut mock = MockWindowManagerPort::new();
        mock.expect_get_state()
            .times(1)
            .returning(move || Ok((BTreeMap::new(), new_monitors.clone(), None)));

        check_and_resync(
            &mock,
            &mut state,
            &["monitoraddedv2>>1,HDMI-A-1,x".to_string()],
            true,
        );

        assert!(state.monitors().contains_key(&MonitorName::new("HDMI-A-1")));
        assert!(!state.monitors().contains_key(&MonitorName::new("DP-1")));
    }

    #[test]
    fn test_check_and_resync_leaves_state_as_is_on_get_state_error() {
        let mut state = inconsistent_state();
        let before = state.clone();

        let mut mock = MockWindowManagerPort::new();
        mock.expect_get_state().times(1).returning(|| {
            Err(WindowManagerError::IpcError {
                reason: "boom".to_string(),
            })
        });

        check_and_resync(&mock, &mut state, &[], false);

        assert_eq!(state, before);
    }
}
