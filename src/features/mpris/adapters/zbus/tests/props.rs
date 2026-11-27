//! How `update_state_from_props` folds a `GetAll` or `PropertiesChanged` map into a player.
use super::fake_bus::{metadata, props, text};
use crate::features::mpris::adapters::zbus::props::update_state_from_props;
use crate::features::mpris::domain::{
    AlbumArtUrl, PlaybackStatus, PlayerName, PlayerState, TrackArtist, TrackName,
};
use crate::shared::dbus::domain::{DBusValue, PropertiesMap};

/// A player named `vlc` playing `Song` by `Artist`, with cover art `cover.png`.
fn full_player() -> PlayerState {
    let mut state = PlayerState::new(PlayerName::new("vlc"));
    state.set_status(PlaybackStatus::Playing);
    state.set_track_name(Some(TrackName::new("Song")));
    state.set_artist(Some(TrackArtist::new("Artist")));
    state.set_album_art(Some(AlbumArtUrl::new("cover.png")));
    state
}

/// `start` after `props` is folded into it.
fn applied(mut start: PlayerState, props: &PropertiesMap) -> PlayerState {
    update_state_from_props(&mut start, props);
    start
}

/// `full_player` with its status set to `status`.
fn player_with_status(status: PlaybackStatus) -> PlayerState {
    let mut state = full_player();
    state.set_status(status);
    state
}

#[test]
fn test_playback_status_maps_each_known_word_and_falls_back_to_unknown() {
    let cases = [
        (PlaybackStatus::Unknown, "Playing", PlaybackStatus::Playing),
        (PlaybackStatus::Unknown, "Paused", PlaybackStatus::Paused),
        (PlaybackStatus::Unknown, "Stopped", PlaybackStatus::Stopped),
        (PlaybackStatus::Playing, "playing", PlaybackStatus::Unknown),
    ];
    for (start, word, expected) in cases {
        let change = props(vec![("PlaybackStatus", text(word))]);
        let state = applied(player_with_status(start), &change);
        assert_eq!(state, player_with_status(expected), "{word}");
    }
}

#[test]
fn test_a_status_that_is_not_a_string_keeps_the_previous_one() {
    let change = props(vec![("PlaybackStatus", DBusValue::Int(1))]);
    assert_eq!(applied(full_player(), &change), full_player());
}

#[test]
fn test_metadata_sets_title_first_artist_and_art_url() {
    let artists = DBusValue::Array(vec![text("First"), text("Second")]);
    let meta = metadata(vec![
        ("xesam:title", text("Title")),
        ("xesam:artist", artists),
        ("mpris:artUrl", text("art.jpg")),
    ]);
    let change = props(vec![("Metadata", meta), ("PlaybackStatus", text("Paused"))]);
    let mut expected = PlayerState::new(PlayerName::new("vlc"));
    expected.set_status(PlaybackStatus::Paused);
    expected.set_track_name(Some(TrackName::new("Title")));
    expected.set_artist(Some(TrackArtist::new("First")));
    expected.set_album_art(Some(AlbumArtUrl::new("art.jpg")));
    let start = PlayerState::new(PlayerName::new("vlc"));
    assert_eq!(applied(start, &change), expected);
}

#[test]
fn test_metadata_entries_of_the_wrong_shape_keep_the_previous_values() {
    let shapes = [
        metadata(vec![
            ("xesam:title", DBusValue::Int(1)),
            ("mpris:artUrl", DBusValue::Null),
        ]),
        metadata(vec![("xesam:artist", DBusValue::Array(vec![]))]),
        metadata(vec![(
            "xesam:artist",
            DBusValue::Array(vec![DBusValue::Int(1), text("B")]),
        )]),
        metadata(vec![("xesam:artist", text("Not an array"))]),
        text("not a dictionary"),
    ];
    for meta in shapes {
        let change = props(vec![("Metadata", meta.clone())]);
        assert_eq!(applied(full_player(), &change), full_player(), "{meta:?}");
    }
}

#[test]
fn test_metadata_without_an_entry_keeps_that_value() {
    let change = props(vec![(
        "Metadata",
        metadata(vec![("xesam:title", text("Next"))]),
    )]);
    let mut expected = full_player();
    expected.set_track_name(Some(TrackName::new("Next")));
    assert_eq!(applied(full_player(), &change), expected);
}
