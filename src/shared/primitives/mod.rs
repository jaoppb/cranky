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
