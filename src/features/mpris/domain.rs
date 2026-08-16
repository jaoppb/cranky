use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerName(String);
impl PlayerName {
    pub fn new(val: impl Into<String>) -> Self { Self(val.into()) }
    pub fn as_str(&self) -> &str { &self.0 }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrackName(String);
impl TrackName {
    pub fn new(val: impl Into<String>) -> Self { Self(val.into()) }
    pub fn as_str(&self) -> &str { &self.0 }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrackArtist(String);
impl TrackArtist {
    pub fn new(val: impl Into<String>) -> Self { Self(val.into()) }
    pub fn as_str(&self) -> &str { &self.0 }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AlbumArtUrl(String);
impl AlbumArtUrl {
    pub fn new(val: impl Into<String>) -> Self { Self(val.into()) }
    pub fn as_str(&self) -> &str { &self.0 }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PlaybackStatus {
    Playing,
    Paused,
    Stopped,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerState {
    name: PlayerName,
    status: PlaybackStatus,
    track_name: Option<TrackName>,
    artist: Option<TrackArtist>,
    album_art: Option<AlbumArtUrl>,
}

impl PlayerState {
    pub fn new(name: PlayerName) -> Self {
        Self {
            name,
            status: PlaybackStatus::Unknown,
            track_name: None,
            artist: None,
            album_art: None,
        }
    }

    pub fn name(&self) -> &PlayerName { &self.name }
    pub fn status(&self) -> &PlaybackStatus { &self.status }
    pub fn track_name(&self) -> Option<&TrackName> { self.track_name.as_ref() }
    pub fn artist(&self) -> Option<&TrackArtist> { self.artist.as_ref() }
    pub fn album_art(&self) -> Option<&AlbumArtUrl> { self.album_art.as_ref() }

    pub fn set_status(&mut self, status: PlaybackStatus) { self.status = status; }
    pub fn set_track_name(&mut self, track_name: Option<TrackName>) { self.track_name = track_name; }
    pub fn set_artist(&mut self, artist: Option<TrackArtist>) { self.artist = artist; }
    pub fn set_album_art(&mut self, album_art: Option<AlbumArtUrl>) { self.album_art = album_art; }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct MprisState {
    pub active_player: Option<PlayerName>,
    pub players: std::collections::HashMap<String, PlayerState>, // Using String key because serde deserialize requires it for keys
}

impl MprisState {
    pub fn active(&self) -> Option<&PlayerState> {
        self.active_player.as_ref().and_then(|name| self.players.get(name.as_str()))
    }
}
