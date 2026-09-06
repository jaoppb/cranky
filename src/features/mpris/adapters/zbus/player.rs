use std::sync::Arc;
use tokio::sync::watch;
use tokio_stream::StreamExt;
use tracing::warn;

use crate::features::mpris::domain::{MprisState, PlayerName, PlayerState};
use crate::shared::dbus::domain::{BusType, Destination, Interface, Path};
use crate::shared::dbus::ports::DbusConnectionPort;

use super::props::update_state_from_props;

pub(crate) async fn add_player(
    conn: &Arc<dyn DbusConnectionPort>,
    mpris_tx: &watch::Sender<MprisState>,
    dest: Destination,
) {
    let path = Path::new("/org/mpris/MediaPlayer2");
    let iface = Interface::new("org.mpris.MediaPlayer2.Player");

    let props = match conn
        .get_all_properties(BusType::Session, &dest, &path, &iface)
        .await
    {
        Ok(p) => p,
        Err(e) => {
            warn!("Failed to read MPRIS properties for {dest:?}: {e}");
            return;
        }
    };

    let player_name = PlayerName::new(
        dest.as_str()
            .strip_prefix("org.mpris.MediaPlayer2.")
            .unwrap_or(dest.as_str()),
    );
    let mut state = PlayerState::new(player_name.clone());

    update_state_from_props(&mut state, &props);

    spawn_property_listener(conn.clone(), mpris_tx.clone(), dest, path);

    let mut mpris_state = mpris_tx.borrow().clone();
    mpris_state
        .players
        .insert(player_name.as_str().to_string(), state);
    if mpris_state.active_player.is_none() {
        mpris_state.active_player = Some(player_name.clone());
    }
    let name_str = player_name.as_str();
    tracing::debug!("Found MPRIS player: {name_str}");
    let _ = mpris_tx.send(mpris_state);
}

pub(crate) fn spawn_property_listener(
    conn: Arc<dyn DbusConnectionPort>,
    tx: watch::Sender<MprisState>,
    dest: Destination,
    path: Path,
) {
    tokio::spawn(async move {
        if let Ok(mut stream) = conn
            .subscribe_properties_changed(BusType::Session, &dest, &path)
            .await
        {
            while let Some((changed_iface, changed_props)) = stream.next().await {
                if changed_iface.as_str() == "org.mpris.MediaPlayer2.Player" {
                    let mut mpris_state = tx.borrow().clone();
                    let p_name = dest
                        .as_str()
                        .strip_prefix("org.mpris.MediaPlayer2.")
                        .unwrap_or(dest.as_str());
                    if let Some(mut player) = mpris_state.players.get(p_name).cloned() {
                        tracing::debug!("MPRIS properties changed for player: {p_name}");
                        update_state_from_props(&mut player, &changed_props);
                        mpris_state.players.insert(p_name.to_string(), player);
                        let _ = tx.send(mpris_state);
                    }
                }
            }
        }
    });
}
