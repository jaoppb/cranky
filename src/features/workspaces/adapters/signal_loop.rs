use super::event_parser::parse_event;
use super::hyprland_provider::HyprlandProvider;
use super::inconsistency::find_state_inconsistencies;
use crate::features::workspaces::ports::WindowManagerPort;
use crate::shared::events::signals::{HyprlandState, SignalHub};
use std::io::{BufRead, ErrorKind};
use std::sync::Arc;

#[allow(clippy::too_many_lines, clippy::useless_let_if_seq)]
pub async fn run_event_loop<P: WindowManagerPort + 'static>(
    adapter: P,
    provider: Arc<dyn HyprlandProvider>,
    hub: Arc<SignalHub>,
) {
    tokio::task::spawn_blocking(move || {
        let hypr_tx = hub.hyprland_tx();

        loop {
            let stream = match provider.listen_events() {
                Ok(s) => s,
                Err(e) => {
                    tracing::error!("Failed to connect to Hyprland event socket: {e}");
                    std::thread::sleep(std::time::Duration::from_secs(1));
                    continue;
                }
            };

            let _ = stream.set_read_timeout(Some(std::time::Duration::from_millis(2)));
            let mut reader = std::io::BufReader::new(stream);

            let mut current_state = match adapter.get_state() {
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
            };

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
                        let trimmed = line.trim();
                        if !trimmed.is_empty() {
                            tracing::trace!(event = trimmed, "Hyprland event received");
                            batch_events.push(trimmed.to_string());
                        }

                        let mut state_changed = false;
                        if let Some(event) = parse_event(&line) {
                            current_state.apply_event(&event);
                            state_changed = true;
                        }

                        let _ = reader
                            .get_mut()
                            .set_read_timeout(Some(std::time::Duration::from_millis(2)));
                        loop {
                            line.clear();
                            match reader.read_line(&mut line) {
                                Ok(0) => break,
                                Ok(_) => {
                                    let trimmed = line.trim();
                                    if !trimmed.is_empty() {
                                        tracing::trace!(
                                            event = trimmed,
                                            "Hyprland event received"
                                        );
                                        batch_events.push(trimmed.to_string());
                                    }
                                    if let Some(event) = parse_event(&line) {
                                        current_state.apply_event(&event);
                                        state_changed = true;
                                    }
                                }
                                Err(e)
                                    if e.kind() == ErrorKind::WouldBlock
                                        || e.kind() == ErrorKind::TimedOut =>
                                {
                                    break;
                                }
                                Err(_) => break,
                            }
                        }

                        if state_changed {
                            let inconsistencies = find_state_inconsistencies(&current_state);

                            if !inconsistencies.is_empty() {
                                tracing::warn!(
                                    reasons = ?inconsistencies,
                                    batch_events = ?batch_events,
                                    "Hyprland state inconsistent after event batch, forcing full resync"
                                );
                                match adapter.get_state() {
                                    Ok((workspaces, monitors, focused)) => {
                                        current_state =
                                            HyprlandState::new(workspaces, monitors, focused);
                                    }
                                    Err(e) => {
                                        tracing::error!(
                                            "Failed to resync Hyprland state after inconsistency: {e}"
                                        );
                                    }
                                }
                            }
                        }

                        if state_changed && *hypr_tx.borrow() != current_state {
                            let _ = hypr_tx.send(current_state.clone());
                        }
                    }
                    Err(e) => {
                        tracing::error!("Hyprland event socket read error: {e}");
                        break;
                    }
                }
            }

            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    });
}
