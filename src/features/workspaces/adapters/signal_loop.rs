use super::event_parser::parse_event;
use super::hyprland_provider::HyprlandProvider;
use super::inconsistency::find_state_inconsistencies;
use crate::features::workspaces::ports::WindowManagerPort;
use crate::shared::events::signals::{HyprlandState, SignalHub};
use std::io::{BufRead, ErrorKind};
use std::sync::Arc;

fn process_line(
    line: &str,
    current_state: &mut HyprlandState,
    batch_events: &mut Vec<String>,
) -> bool {
    let trimmed = line.trim();
    if !trimmed.is_empty() {
        tracing::trace!(event = trimmed, "Hyprland event received");
        batch_events.push(trimmed.to_string());
    }
    parse_event(line).is_some_and(|event| {
        current_state.apply_event(&event);
        true
    })
}

fn drain_batch<R: BufRead>(
    reader: &mut R,
    line_buf: &mut String,
    current_state: &mut HyprlandState,
    batch_events: &mut Vec<String>,
) -> bool {
    let mut state_changed = process_line(line_buf, current_state, batch_events);
    loop {
        line_buf.clear();
        match reader.read_line(line_buf) {
            Ok(0) => break,
            Ok(_) => {
                if process_line(line_buf, current_state, batch_events) {
                    state_changed = true;
                }
            }
            Err(e) if e.kind() == ErrorKind::WouldBlock || e.kind() == ErrorKind::TimedOut => {
                break;
            }
            Err(_) => break,
        }
    }
    state_changed
}

fn check_and_resync<P: WindowManagerPort>(
    adapter: &P,
    current_state: &mut HyprlandState,
    batch_events: &[String],
) {
    let inconsistencies = find_state_inconsistencies(current_state);
    if !inconsistencies.is_empty() {
        tracing::warn!(
            reasons = ?inconsistencies,
            batch_events = ?batch_events,
            "Hyprland state inconsistent after event batch, forcing full resync"
        );
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

                let state_changed =
                    drain_batch(&mut reader, &mut line, &mut current_state, &mut batch_events);

                if state_changed {
                    check_and_resync(adapter, &mut current_state, &batch_events);
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
