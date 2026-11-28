use std::sync::Arc;
use tokio::sync::watch;
use tokio_stream::StreamExt;
use tracing::warn;

use crate::features::mpris::domain::{MprisState, PlayerName, PlayerState};
use crate::shared::dbus::domain::{
    BusObject, BusType, Destination, Interface, Path, PropertiesMap,
};
use crate::shared::dbus::ports::{DbusConnectionError, DbusConnectionPort};

use super::props::update_state_from_props;

pub(super) async fn add_player(
    conn: &Arc<dyn DbusConnectionPort>,
    mpris_tx: &watch::Sender<MprisState>,
    dest: Destination,
) {
    let object = BusObject::new(BusType::Session, dest, Path::new("/org/mpris/MediaPlayer2"));
    let Some(props) = read_player_properties(conn, &object).await else {
        return;
    };

    let dest = object.destination().as_str();
    let player_name = PlayerName::new(dest.strip_prefix("org.mpris.MediaPlayer2.").unwrap_or(dest));
    let mut state = PlayerState::new(player_name.clone());

    update_state_from_props(&mut state, &props);

    spawn_property_listener(conn.clone(), mpris_tx.clone(), object);

    publish_player(mpris_tx, &player_name, state);
}

/// Reads every property of `object`'s player interface, logging and giving up when the
/// read fails.
async fn read_player_properties(
    conn: &Arc<dyn DbusConnectionPort>,
    object: &BusObject,
) -> Option<PropertiesMap> {
    let iface = Interface::new("org.mpris.MediaPlayer2.Player");
    match conn.get_all_properties(object, &iface).await {
        Ok(properties) => Some(properties),
        Err(error) => {
            log_read_failed(object.destination(), &error);
            None
        }
    }
}

/// Logs that the player at `dest` could not be read.
fn log_read_failed(dest: &Destination, error: &DbusConnectionError) {
    warn!("Failed to read MPRIS properties for {dest:?}: {error}");
}

/// Adds `state` under `player_name` to the published MPRIS state, making it the active
/// player when none is, and publishes the result.
fn publish_player(
    mpris_tx: &watch::Sender<MprisState>,
    player_name: &PlayerName,
    state: PlayerState,
) {
    let mut mpris_state = mpris_tx.borrow().clone();
    mpris_state
        .players
        .insert(player_name.as_str().to_string(), state);
    if mpris_state.active_player.is_none() {
        mpris_state.active_player = Some(player_name.clone());
    }
    log_player_found(player_name.as_str());
    let _ = mpris_tx.send(mpris_state);
}

/// Logs that the player `name_str` was found.
fn log_player_found(name_str: &str) {
    tracing::debug!("Found MPRIS player: {name_str}");
}

fn spawn_property_listener(
    conn: Arc<dyn DbusConnectionPort>,
    tx: watch::Sender<MprisState>,
    object: BusObject,
) {
    tokio::spawn(async move {
        let Ok(stream) = conn.subscribe_properties_changed(&object).await else {
            return;
        };
        let dest = object.destination();
        let mut player_changes = stream
            .filter(|(changed_iface, _)| changed_iface.as_str() == "org.mpris.MediaPlayer2.Player");
        while let Some((_, changed_props)) = player_changes.next().await {
            apply_player_change(&tx, dest, &changed_props);
        }
    });
}

/// Applies a change of the player interface at `dest` to its published player and publishes
/// the result; a player no longer published is left out.
fn apply_player_change(
    tx: &watch::Sender<MprisState>,
    dest: &Destination,
    changed_props: &PropertiesMap,
) {
    let mut mpris_state = tx.borrow().clone();
    let p_name = dest
        .as_str()
        .strip_prefix("org.mpris.MediaPlayer2.")
        .unwrap_or(dest.as_str());
    let Some(mut player) = mpris_state.players.get(p_name).cloned() else {
        return;
    };
    tracing::debug!("MPRIS properties changed for player: {p_name}");
    update_state_from_props(&mut player, changed_props);
    mpris_state.players.insert(p_name.to_string(), player);
    let _ = tx.send(mpris_state);
}
