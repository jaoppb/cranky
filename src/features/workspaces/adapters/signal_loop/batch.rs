//! Reads one batch of event lines and applies each to the state.
use super::{EventBatch, LineOutcome, log_event_received};
use crate::features::workspaces::adapters::event_parser::parse_event;
use crate::shared::events::core::WindowManagerEvent;
use crate::shared::events::signals::HyprlandState;
use std::io::{BufRead, ErrorKind};

fn process_line(
    line: &str,
    current_state: &mut HyprlandState,
    batch_events: &mut Vec<String>,
) -> LineOutcome {
    record_line(line, batch_events);
    let Some(event) = parse_event(line) else {
        return LineOutcome::Ignored;
    };
    current_state.apply_event(&event);
    if matches!(event, WindowManagerEvent::MonitorAdded { .. }) {
        LineOutcome::AppliedNeedsResync
    } else {
        LineOutcome::Applied
    }
}

/// Traces a non-blank line and keeps it, trimmed, among the batch's events.
fn record_line(line: &str, batch_events: &mut Vec<String>) {
    let trimmed = line.trim();
    if !trimmed.is_empty() {
        log_event_received(trimmed);
        batch_events.push(trimmed.to_string());
    }
}

pub(super) fn drain_batch<R: BufRead>(
    reader: &mut R,
    line_buf: &mut String,
    current_state: &mut HyprlandState,
) -> EventBatch {
    let mut batch_events = Vec::new();
    let mut outcome = process_line(line_buf, current_state, &mut batch_events);
    loop {
        line_buf.clear();
        match reader.read_line(line_buf) {
            Ok(0) => break,
            Ok(_) => {
                outcome = outcome.max(process_line(line_buf, current_state, &mut batch_events));
            }
            Err(error)
                if error.kind() == ErrorKind::WouldBlock || error.kind() == ErrorKind::TimedOut =>
            {
                break;
            }
            Err(_) => break,
        }
    }
    EventBatch {
        events: batch_events,
        outcome,
    }
}
