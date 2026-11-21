//! Newtypes for the strings that identify a tray item on the bus and describe it.
use serde::{Deserialize, Serialize};

/// Key of a tray item in the registry and in the hub's state: its bus name followed by its object
/// path, so it is unique across items.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(transparent)]
pub struct SystrayId(String);

impl SystrayId {
    /// Wraps an id as given, without checking its shape.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
    #[must_use]
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for SystrayId {
    fn from(id: &str) -> Self {
        Self::new(id)
    }
}

impl From<String> for SystrayId {
    fn from(id: String) -> Self {
        Self::new(id)
    }
}

/// D-Bus bus name that owns a tray item, used to address calls to it and to drop the item when
/// that name loses its owner.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(transparent)]
pub struct Destination(String);

impl Destination {
    pub(crate) fn new(dest: impl Into<String>) -> Self {
        Self(dest.into())
    }
    #[must_use]
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(transparent)]
pub(crate) struct ObjectPath(String);

impl ObjectPath {
    pub(crate) fn new(path: impl Into<String>) -> Self {
        Self(path.into())
    }
    #[must_use]
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(transparent)]
pub(crate) struct Title(String);

impl Title {
    pub(crate) fn new(title: impl Into<String>) -> Self {
        Self(title.into())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(transparent)]
pub(crate) struct ItemId(String);

impl ItemId {
    pub(crate) fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
    #[cfg(test)]
    #[must_use]
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(transparent)]
pub(crate) struct WindowId(u32);

impl WindowId {
    #[must_use]
    pub(crate) const fn new(id: u32) -> Self {
        Self(id)
    }
    #[cfg(test)]
    #[must_use]
    pub(crate) const fn value(self) -> u32 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(transparent)]
pub(crate) struct ItemIsMenu(bool);

impl ItemIsMenu {
    #[must_use]
    pub(crate) const fn new(val: bool) -> Self {
        Self(val)
    }
    #[must_use]
    pub(crate) const fn value(self) -> bool {
        self.0
    }
}
