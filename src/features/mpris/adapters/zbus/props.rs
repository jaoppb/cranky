use crate::features::mpris::domain::{
    AlbumArtUrl, PlaybackStatus, PlayerState, TrackArtist, TrackName,
};
use crate::shared::dbus::domain::{DBusValue, PropertiesMap, PropertyName};

pub(crate) fn update_state_from_props(
    state: &mut PlayerState,
    props: &PropertiesMap,
) {
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
            && let Some(DBusValue::String(artist)) = a.first()
        {
            state.set_artist(Some(TrackArtist::new(artist)));
        }
        if let Some(DBusValue::String(url)) = meta.get("mpris:artUrl") {
            state.set_album_art(Some(AlbumArtUrl::new(url)));
        }
    }
}
