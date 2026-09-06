use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BusType {
    Session,
    System,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Destination(String);

impl Destination {
    #[must_use]
    pub fn new(val: impl Into<String>) -> Self {
        Self(val.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Path(String);

impl Path {
    #[must_use]
    pub fn new(val: impl Into<String>) -> Self {
        Self(val.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Interface(String);

impl Interface {
    #[must_use]
    pub fn new(val: impl Into<String>) -> Self {
        Self(val.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Member(String);

impl Member {
    #[must_use]
    pub fn new(val: impl Into<String>) -> Self {
        Self(val.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PropertyName(String);

impl PropertyName {
    #[must_use]
    pub fn new(val: impl Into<String>) -> Self {
        Self(val.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
