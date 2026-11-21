use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct PlayerName(String);
impl PlayerName {
    #[must_use]
    pub(crate) fn new(val: impl Into<String>) -> Self {
        Self(val.into())
    }
    #[must_use]
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct TrackName(String);
impl TrackName {
    #[must_use]
    pub(crate) fn new(val: impl Into<String>) -> Self {
        Self(val.into())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct TrackArtist(String);
impl TrackArtist {
    #[must_use]
    pub(crate) fn new(val: impl Into<String>) -> Self {
        Self(val.into())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct AlbumArtUrl(String);
impl AlbumArtUrl {
    #[must_use]
    pub(crate) fn new(val: impl Into<String>) -> Self {
        Self(val.into())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum PlaybackStatus {
    Playing,
    Paused,
    Stopped,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct PlayerState {
    name: PlayerName,
    status: PlaybackStatus,
    track_name: Option<TrackName>,
    artist: Option<TrackArtist>,
    album_art: Option<AlbumArtUrl>,
}

impl PlayerState {
    #[must_use]
    pub(crate) const fn new(name: PlayerName) -> Self {
        Self {
            name,
            status: PlaybackStatus::Unknown,
            track_name: None,
            artist: None,
            album_art: None,
        }
    }

    pub(crate) const fn set_status(&mut self, status: PlaybackStatus) {
        self.status = status;
    }
    pub(crate) fn set_track_name(&mut self, track_name: Option<TrackName>) {
        self.track_name = track_name;
    }
    pub(crate) fn set_artist(&mut self, artist: Option<TrackArtist>) {
        self.artist = artist;
    }
    pub(crate) fn set_album_art(&mut self, album_art: Option<AlbumArtUrl>) {
        self.album_art = album_art;
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub(crate) struct MprisState {
    pub(crate) active_player: Option<PlayerName>,
    pub(crate) players: std::collections::HashMap<String, PlayerState>, // Using String key because serde deserialize requires it for keys
}

impl MprisState {}
