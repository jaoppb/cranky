pub mod color;
pub mod geometry;
pub mod render;

use serde::Deserialize;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
pub struct ModuleId(u32);

impl ModuleId {
    pub fn new(id: u32) -> Self { Self(id) }
}
impl fmt::Display for ModuleId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{}", self.0) }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
pub struct MonitorId(String);

impl MonitorId {
    pub fn new(id: impl Into<String>) -> Self { Self(id.into()) }
    pub fn as_str(&self) -> &str { &self.0 }
}
impl fmt::Display for MonitorId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{}", self.0) }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
pub struct ModuleName(String);

impl ModuleName {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for ModuleName {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for ModuleName {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl PartialEq<str> for ModuleName {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}

impl PartialEq<&str> for ModuleName {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}

impl fmt::Display for ModuleName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
pub struct FunctionName(String);

impl FunctionName {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for FunctionName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
