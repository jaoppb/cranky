//! Typed wrappers for bus names, object paths, interfaces, members and property names.

use serde::{Deserialize, Serialize};

/// Which message bus a request or subscription targets; scripts spell it `"session"` or `"system"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BusType {
    /// The per-user session bus (media players, notifications, tray items).
    Session,
    /// The machine-wide system bus (hardware and system services).
    System,
}

/// A well-known or unique bus name a message is sent to or comes from, such as
/// `org.mpris.MediaPlayer2.spotify`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Destination(String);

impl Destination {
    /// Wraps a bus name as given; the string is not validated.
    #[must_use]
    pub fn new(val: impl Into<String>) -> Self {
        Self(val.into())
    }

    /// The bus name, for matching and passing to zbus.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// An object path on a bus, such as `/org/mpris/MediaPlayer2`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Path(String);

impl Path {
    /// Wraps an object path as given; the string is not validated.
    #[must_use]
    pub fn new(val: impl Into<String>) -> Self {
        Self(val.into())
    }

    #[must_use]
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

/// A `DBus` interface name, such as `org.mpris.MediaPlayer2.Player`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Interface(String);

impl Interface {
    /// Wraps an interface name as given; the string is not validated.
    #[must_use]
    pub fn new(val: impl Into<String>) -> Self {
        Self(val.into())
    }

    #[must_use]
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

/// A method or signal name on an interface, such as `PropertiesChanged`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Member(String);

impl Member {
    /// Wraps a member name as given; the string is not validated.
    #[must_use]
    pub fn new(val: impl Into<String>) -> Self {
        Self(val.into())
    }

    #[must_use]
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

/// The name of a property on an interface, as returned by `GetAll` or `PropertiesChanged`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PropertyName(String);

impl PropertyName {
    /// Wraps a property name as given; the string is not validated.
    #[must_use]
    pub fn new(val: impl Into<String>) -> Self {
        Self(val.into())
    }

    #[must_use]
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}
