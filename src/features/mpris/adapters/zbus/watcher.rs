use std::sync::Arc;
use tokio::sync::watch;
use tokio_stream::StreamExt;

use crate::features::mpris::domain::{MprisState, PlayerName};
use crate::shared::dbus::domain::BusType;
use crate::shared::dbus::ports::{DbusConnectionError, DbusConnectionPort};
use crate::shared::events::signals::SignalHub;

use super::player::add_player;

pub struct ZbusMprisAdapter {
    conn: Arc<dyn DbusConnectionPort>,
    mpris_tx: watch::Sender<MprisState>,
}

impl ZbusMprisAdapter {
    #[must_use]
    pub fn new(conn: Arc<dyn DbusConnectionPort>, hub: &SignalHub) -> Self {
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
    pub async fn start_watching(&self) -> Result<(), DbusConnectionError> {
        tracing::debug!("Starting MPRIS watcher...");
        self.load_initial_players().await?;
        self.watch_name_changes().await?;
        Ok(())
    }

    async fn load_initial_players(&self) -> Result<(), DbusConnectionError> {
        let names = self.conn.list_names(BusType::Session).await?;
        for name in names {
            if name.as_str().starts_with("org.mpris.MediaPlayer2.") {
                add_player(&self.conn, &self.mpris_tx, name).await;
            }
        }
        Ok(())
    }

    async fn watch_name_changes(&self) -> Result<(), DbusConnectionError> {
        let mut stream = self.conn.subscribe_name_changes(BusType::Session).await?;
        let tx = self.mpris_tx.clone();
        let conn = self.conn.clone();

        tokio::spawn(async move {
            while let Some(event) = stream.next().await {
                let name = event.name().as_str();
                if name.starts_with("org.mpris.MediaPlayer2.") {
                    let player_name = name
                        .strip_prefix("org.mpris.MediaPlayer2.")
                        .unwrap_or(name)
                        .to_string();

                    if event.is_new() {
                        add_player(&conn, &tx, event.name().clone()).await;
                    } else if event.is_gone() {
                        tracing::debug!("MPRIS player gone: {player_name}");
                        let mut mpris_state = tx.borrow().clone();
                        mpris_state.players.remove(&player_name);
                        if let Some(active) = &mpris_state.active_player
                            && active.as_str() == player_name
                        {
                            mpris_state.active_player =
                                mpris_state.players.keys().next().map(PlayerName::new);
                        }
                        let _ = tx.send(mpris_state);
                    }
                }
            }
        });

        Ok(())
    }
}
