use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(transparent)]
pub struct SystrayId(String);

impl SystrayId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for SystrayId {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for SystrayId {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(transparent)]
pub struct Destination(String);

impl Destination {
    pub fn new(dest: impl Into<String>) -> Self {
        Self(dest.into())
    }
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(transparent)]
pub struct ObjectPath(String);

impl ObjectPath {
    pub fn new(path: impl Into<String>) -> Self {
        Self(path.into())
    }
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(transparent)]
pub struct Title(String);

impl Title {
    pub fn new(title: impl Into<String>) -> Self {
        Self(title.into())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(transparent)]
pub struct ItemId(String);

impl ItemId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
    #[cfg(test)]
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(transparent)]
pub struct WindowId(u32);

impl WindowId {
    #[must_use]
    pub const fn new(id: u32) -> Self {
        Self(id)
    }
    #[cfg(test)]
    #[must_use]
    pub const fn value(&self) -> u32 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(transparent)]
pub struct ItemIsMenu(bool);

impl ItemIsMenu {
    #[must_use]
    pub const fn new(val: bool) -> Self {
        Self(val)
    }
    #[must_use]
    pub const fn value(&self) -> bool {
        self.0
    }
}
