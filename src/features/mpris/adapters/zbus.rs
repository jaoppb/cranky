use std::sync::Arc;
use tokio::sync::watch;
use tokio_stream::StreamExt;
use tracing::{info, warn};

use crate::shared::dbus::domain::{BusType, DBusValue, Destination, Interface, Path};
use crate::shared::dbus::ports::{DbusConnectionError, DbusConnectionPort};
use crate::shared::events::signals::SignalHub;
use crate::features::mpris::domain::*;

pub struct ZbusMprisAdapter {
    conn: Arc<dyn DbusConnectionPort>,
    mpris_tx: watch::Sender<MprisState>,
}

impl ZbusMprisAdapter {
    pub fn new(conn: Arc<dyn DbusConnectionPort>, hub: &SignalHub) -> Self {
        Self {
            conn,
            mpris_tx: hub.mpris_tx(),
        }
    }

    pub async fn start_watching(&mut self) -> Result<(), DbusConnectionError> {
        info!("Starting MPRIS watcher...");
        self.load_initial_players().await?;
        self.watch_name_changes().await?;
        Ok(())
    }

    async fn load_initial_players(&mut self) -> Result<(), DbusConnectionError> {
        let names = self.conn.list_names(BusType::Session).await?;
        for name in names {
            if name.as_str().starts_with("org.mpris.MediaPlayer2.") {
                self.add_player(name).await;
            }
        }
        Ok(())
    }

    async fn add_player(&mut self, dest: Destination) {
        let path = Path::new("/org/mpris/MediaPlayer2");
        let iface = Interface::new("org.mpris.MediaPlayer2.Player");

        let props = match self.conn.get_all_properties(BusType::Session, &dest, &path, &iface).await {
            Ok(p) => p,
            Err(e) => {
                warn!("Failed to read MPRIS properties for {:?}: {}", dest, e);
                return;
            }
        };

        let player_name = PlayerName::new(dest.as_str().strip_prefix("org.mpris.MediaPlayer2.").unwrap_or(dest.as_str()));
        let mut state = PlayerState::new(player_name.clone());

        Self::update_state_from_props(&mut state, &props);

        // Sub to properties changed
        if let Ok(mut stream) = self.conn.subscribe_properties_changed(BusType::Session, &dest, &path).await {
            let tx = self.mpris_tx.clone();
            let dest_clone = dest.clone();
            tokio::spawn(async move {
                while let Some((changed_iface, changed_props)) = stream.next().await {
                    if changed_iface.as_str() == "org.mpris.MediaPlayer2.Player" {
                        let mut mpris_state = tx.borrow().clone();
                        let p_name = dest_clone.as_str().strip_prefix("org.mpris.MediaPlayer2.").unwrap_or(dest_clone.as_str());
                        if let Some(mut player) = mpris_state.players.get(p_name).cloned() {
                            tracing::debug!("MPRIS properties changed for player: {}", p_name);
                            Self::update_state_from_props(&mut player, &changed_props);
                            mpris_state.players.insert(p_name.to_string(), player);
                            let _ = tx.send(mpris_state);
                        }
                    }
                }
            });
        }

        let mut mpris_state = self.mpris_tx.borrow().clone();
        mpris_state.players.insert(player_name.as_str().to_string(), state);
        if mpris_state.active_player.is_none() {
            mpris_state.active_player = Some(player_name.clone());
        }
        tracing::info!("Found MPRIS player: {}", player_name.as_str());
        let _ = self.mpris_tx.send(mpris_state);
    }

    fn update_state_from_props(state: &mut PlayerState, props: &crate::shared::dbus::domain::PropertiesMap) {
        use crate::shared::dbus::domain::PropertyName;

        if let Some(DBusValue::String(s)) = props.get(&PropertyName::new("PlaybackStatus")) {
            state.set_status(match s.as_str() {
                "Playing" => PlaybackStatus::Playing,
                "Paused" => PlaybackStatus::Paused,
                "Stopped" => PlaybackStatus::Stopped,
                _ => PlaybackStatus::Unknown,
            });
        }

        if let Some(DBusValue::Dict(meta)) = props.get(&PropertyName::new("Metadata")) {
            if let Some(DBusValue::String(t)) = meta.get("xesam:title") {
                state.set_track_name(Some(TrackName::new(t)));
            }
            if let Some(DBusValue::Array(a)) = meta.get("xesam:artist")
                && let Some(DBusValue::String(artist)) = a.first() {
                    state.set_artist(Some(TrackArtist::new(artist)));
                }
            if let Some(DBusValue::String(url)) = meta.get("mpris:artUrl") {
                state.set_album_art(Some(AlbumArtUrl::new(url)));
            }
        }
    }

    async fn watch_name_changes(&mut self) -> Result<(), DbusConnectionError> {
        let mut stream = self.conn.subscribe_name_changes(BusType::Session).await?;
        
        // Use an Arc<Mutex<ZbusMprisAdapter>>? We can't really do that since we're consuming stream in a spawn.
        // Let's pass the conn and tx to the spawned task.
        let tx = self.mpris_tx.clone();
        let conn = self.conn.clone();
        
        tokio::spawn(async move {
            while let Some(event) = stream.next().await {
                let name = event.name().as_str();
                if name.starts_with("org.mpris.MediaPlayer2.") {
                    let player_name = name.strip_prefix("org.mpris.MediaPlayer2.").unwrap_or(name).to_string();
                    
                    if event.is_new() {
                        // We need to add the player. Since add_player requires self.conn, we have to duplicate its logic or create a helper.
                        // For simplicity, we just send a signal to a channel? Or just implement it here:
                        let dest = event.name().clone();
                        let path = Path::new("/org/mpris/MediaPlayer2");
                        let iface = Interface::new("org.mpris.MediaPlayer2.Player");
                        
                        if let Ok(props) = conn.get_all_properties(BusType::Session, &dest, &path, &iface).await {
                            let mut state = PlayerState::new(PlayerName::new(&player_name));
                            Self::update_state_from_props(&mut state, &props);
                            
                            if let Ok(mut prop_stream) = conn.subscribe_properties_changed(BusType::Session, &dest, &path).await {
                                let tx_clone = tx.clone();
                                let dest_clone = dest.clone();
                                tokio::spawn(async move {
                                    while let Some((changed_iface, changed_props)) = prop_stream.next().await {
                                        if changed_iface.as_str() == "org.mpris.MediaPlayer2.Player" {
                                            let mut mpris_state = tx_clone.borrow().clone();
                                            let p_name = dest_clone.as_str().strip_prefix("org.mpris.MediaPlayer2.").unwrap_or(dest_clone.as_str());
                                            if let Some(mut player) = mpris_state.players.get(p_name).cloned() {
                                                Self::update_state_from_props(&mut player, &changed_props);
                                                mpris_state.players.insert(p_name.to_string(), player);
                                                let _ = tx_clone.send(mpris_state);
                                            }
                                        }
                                    }
                                });
                            }
                            
                            let mut mpris_state = tx.borrow().clone();
                            mpris_state.players.insert(player_name.clone(), state);
                            if mpris_state.active_player.is_none() {
                                mpris_state.active_player = Some(PlayerName::new(&player_name));
                            }
                            tracing::info!("Found new MPRIS player: {}", player_name);
                            let _ = tx.send(mpris_state);
                        }
                    } else if event.is_gone() {
                        tracing::info!("MPRIS player gone: {}", player_name);
                        let mut mpris_state = tx.borrow().clone();
                        mpris_state.players.remove(&player_name);
                        if let Some(active) = &mpris_state.active_player
                            && active.as_str() == player_name {
                                mpris_state.active_player = mpris_state.players.keys().next().map(PlayerName::new);
                            }
                        let _ = tx.send(mpris_state);
                    }
                }
            }
        });

        Ok(())
    }
}
