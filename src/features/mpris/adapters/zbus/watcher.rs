//! Watches the session bus for MPRIS players appearing and leaving, keeping the hub's MPRIS
//! state and active player up to date.
use std::sync::Arc;
use tokio::sync::watch;
use tokio_stream::StreamExt;

use crate::features::mpris::domain::{MprisState, PlayerName};
use crate::shared::dbus::domain::{BusType, NameChangedStream, NameOwnerChanged};
use crate::shared::dbus::ports::{DbusConnectionError, DbusConnectionPort};
use crate::shared::events::signals::SignalHub;

use super::player::add_player;

pub(crate) struct ZbusMprisAdapter {
    conn: Arc<dyn DbusConnectionPort>,
    mpris_tx: watch::Sender<MprisState>,
}

impl ZbusMprisAdapter {
    #[must_use]
    pub(crate) fn new(conn: Arc<dyn DbusConnectionPort>, hub: &SignalHub) -> Self {
        Self {
            conn,
            mpris_tx: hub.mpris_tx(),
        }
    }

    /// Starts watching for MPRIS players and their playback state changes.
    ///
    /// # Errors
    ///
    /// Returns [`DbusConnectionError`] if subscribing or querying D-Bus fails.
    pub(crate) async fn start_watching(&self) -> Result<(), DbusConnectionError> {
        log_watcher_starting();
        self.load_initial_players().await?;
        self.watch_name_changes().await?;
        Ok(())
    }

    async fn load_initial_players(&self) -> Result<(), DbusConnectionError> {
        let names = self.conn.list_names(BusType::Session).await?;
        let players = names
            .into_iter()
            .filter(|name| name.as_str().starts_with("org.mpris.MediaPlayer2."));
        for name in players {
            add_player(&self.conn, &self.mpris_tx, name).await;
        }
        Ok(())
    }

    async fn watch_name_changes(&self) -> Result<(), DbusConnectionError> {
        let stream = self.conn.subscribe_name_changes(BusType::Session).await?;
        let tx = self.mpris_tx.clone();
        let conn = self.conn.clone();

        tokio::spawn(follow_name_changes(stream, conn, tx));

        Ok(())
    }
}

/// Logs that the watcher is starting.
fn log_watcher_starting() {
    tracing::debug!("Starting MPRIS watcher...");
}

/// Applies each name change `stream` yields to the MPRIS state, until the stream ends.
async fn follow_name_changes(
    mut stream: NameChangedStream,
    conn: Arc<dyn DbusConnectionPort>,
    tx: watch::Sender<MprisState>,
) {
    while let Some(event) = stream.next().await {
        apply_name_change(&conn, &tx, event).await;
    }
}

/// Adds the MPRIS player whose name `event` shows appearing, or removes the one it shows
/// leaving; other names, and owner changes, are ignored.
async fn apply_name_change(
    conn: &Arc<dyn DbusConnectionPort>,
    tx: &watch::Sender<MprisState>,
    event: NameOwnerChanged,
) {
    let name = event.name().as_str();
    if !name.starts_with("org.mpris.MediaPlayer2.") {
        return;
    }
    let player_name = name
        .strip_prefix("org.mpris.MediaPlayer2.")
        .unwrap_or(name)
        .to_string();

    if event.is_new() {
        add_player(conn, tx, event.name().clone()).await;
    } else if event.is_gone() {
        remove_player(tx, &player_name);
    }
}

/// Removes `player_name` from the published state, handing the active role to a remaining
/// player when it held it, and publishes the result.
fn remove_player(tx: &watch::Sender<MprisState>, player_name: &str) {
    log_player_gone(player_name);
    let mut mpris_state = tx.borrow().clone();
    mpris_state.players.remove(player_name);
    if let Some(active) = &mpris_state.active_player
        && active.as_str() == player_name
    {
        mpris_state.active_player = mpris_state.players.keys().next().map(PlayerName::new);
    }
    let _ = tx.send(mpris_state);
}

/// Logs that the player `player_name` left the bus.
fn log_player_gone(player_name: &str) {
    tracing::debug!("MPRIS player gone: {player_name}");
}
