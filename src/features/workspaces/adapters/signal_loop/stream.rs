//! Follows one connection to the event socket until it closes or fails.
use super::batch::drain_batch;
use super::resync::{check_and_resync, fetch_initial_state, publish_if_changed};
use super::{LineOutcome, log_read_failed, log_socket_closed};
use crate::features::workspaces::ports::WindowManagerPort;
use crate::shared::events::signals::HyprlandState;
use std::io::{BufRead, BufReader};
use std::os::unix::net::UnixStream;
use std::time::Duration;
use tokio::sync::watch;

/// How long a batch waits for the next line before it ends.
const BATCH_READ_TIMEOUT: Duration = Duration::from_millis(2);

/// One connection to the event socket: its reader and line buffer, the state its events keep,
/// and the window manager and hub that state is resynced from and published to.
struct EventStream<'a, P> {
    reader: BufReader<UnixStream>,
    line: String,
    state: HyprlandState,
    adapter: &'a P,
    hypr_tx: &'a watch::Sender<HyprlandState>,
}

pub(super) fn process_event_stream<P: WindowManagerPort>(
    stream: UnixStream,
    adapter: &P,
    hypr_tx: &watch::Sender<HyprlandState>,
) {
    let _ = stream.set_read_timeout(Some(BATCH_READ_TIMEOUT));
    let reader = BufReader::new(stream);
    let state = fetch_initial_state(adapter, hypr_tx);
    let mut events = EventStream {
        reader,
        line: String::new(),
        state,
        adapter,
        hypr_tx,
    };
    events.run();
}

impl<P: WindowManagerPort> EventStream<'_, P> {
    /// Waits for each batch's first line with no timeout, and handles the batch, until the
    /// socket closes or a read fails.
    fn run(&mut self) {
        while self.read_batch_start() {
            self.handle_batch();
        }
    }

    /// Reads a batch's first line with no timeout; false, after logging why, when the socket
    /// closed or the read failed.
    fn read_batch_start(&mut self) -> bool {
        self.line.clear();
        let _ = self.reader.get_mut().set_read_timeout(None);
        match self.reader.read_line(&mut self.line) {
            Ok(0) => {
                log_socket_closed();
                false
            }
            Ok(_) => true,
            Err(error) => {
                log_read_failed(&error);
                false
            }
        }
    }

    /// Drains the batch the first line starts, resyncs the state when it must, and publishes it
    /// if it changed; a batch with no parsed event changes nothing.
    fn handle_batch(&mut self) {
        let _ = self
            .reader
            .get_mut()
            .set_read_timeout(Some(BATCH_READ_TIMEOUT));
        let batch = drain_batch(&mut self.reader, &mut self.line, &mut self.state);
        if batch.outcome == LineOutcome::Ignored {
            return;
        }
        check_and_resync(self.adapter, &mut self.state, &batch);
        publish_if_changed(self.hypr_tx, &self.state);
    }
}
