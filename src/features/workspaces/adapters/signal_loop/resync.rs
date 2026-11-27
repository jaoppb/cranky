//! Brings the state back in line with the window manager, and publishes it to the hub.
use super::{
    EventBatch, LineOutcome, log_forced_resync, log_inconsistent_state, log_initial_fetch_failed,
    log_resync_failed,
};
use crate::features::workspaces::adapters::inconsistency::find_state_inconsistencies;
use crate::features::workspaces::ports::WindowManagerPort;
use crate::shared::events::signals::HyprlandState;
use std::collections::BTreeMap;
use tokio::sync::watch;

pub(super) fn check_and_resync<P: WindowManagerPort>(
    adapter: &P,
    current_state: &mut HyprlandState,
    batch: &EventBatch,
) {
    let inconsistencies = find_state_inconsistencies(current_state);
    if batch.outcome != LineOutcome::AppliedNeedsResync && inconsistencies.is_empty() {
        return;
    }
    if inconsistencies.is_empty() {
        log_forced_resync(&batch.events);
    } else {
        log_inconsistent_state(&inconsistencies, &batch.events);
    }
    resync(adapter, current_state);
}

/// Replaces `current_state` with the window manager's, or logs why it could not.
fn resync<P: WindowManagerPort>(adapter: &P, current_state: &mut HyprlandState) {
    match adapter.get_state() {
        Ok((workspaces, monitors, focused)) => {
            *current_state = HyprlandState::new(workspaces, monitors, focused);
        }
        Err(error) => log_resync_failed(&error),
    }
}

pub(super) fn fetch_initial_state<P: WindowManagerPort>(
    adapter: &P,
    hypr_tx: &watch::Sender<HyprlandState>,
) -> HyprlandState {
    match adapter.get_state() {
        Ok((workspaces, monitors, focused)) => {
            let state = HyprlandState::new(workspaces, monitors, focused);
            publish_if_changed(hypr_tx, &state);
            state
        }
        Err(error) => {
            log_initial_fetch_failed(&error);
            HyprlandState::new(BTreeMap::new(), BTreeMap::new(), None)
        }
    }
}

/// Sends `state` to the hub unless the hub already holds an equal one.
pub(super) fn publish_if_changed(hypr_tx: &watch::Sender<HyprlandState>, state: &HyprlandState) {
    if *hypr_tx.borrow() != *state {
        let _ = hypr_tx.send(state.clone());
    }
}
