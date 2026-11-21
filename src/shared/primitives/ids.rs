use serde::{Deserialize, Serialize};
use std::fmt;

/// Runtime handle of one loaded module instance, minted by the registry from a counter.
/// Each embedding site gets its own id, so a module embedded twice has two.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ModuleId(u32);

impl ModuleId {
    /// Wraps a raw id; only the registry should mint new ones.
    #[must_use]
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) const fn value(self) -> u32 {
        self.0
    }
}

impl fmt::Display for ModuleId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A monitor, named by its Wayland output name (such as `DP-1`); per-monitor state is keyed by it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MonitorId(String);

impl MonitorId {
    /// Wraps an output name as given.
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    #[must_use]
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for MonitorId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Name of a module's script, as used in `[modules]`, `ui.module(name)` and the script file name.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ModuleName(String);

impl ModuleName {
    #[must_use]
    pub(crate) fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    #[must_use]
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for ModuleName {
    fn from(name: String) -> Self {
        Self(name)
    }
}

impl From<&str> for ModuleName {
    fn from(name: &str) -> Self {
        Self(name.to_string())
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

/// Unique identifier for an instance of a module within a container
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ModuleInstanceId(String);

impl ModuleInstanceId {
    #[must_use]
    pub(crate) fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ModuleInstanceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for ModuleInstanceId {
    fn from(identifier: String) -> Self {
        Self(identifier)
    }
}

impl From<&str> for ModuleInstanceId {
    fn from(identifier: &str) -> Self {
        Self(identifier.to_string())
    }
}

/// Name of a global function in a module's script, called by `ui.action.call` actions and by host
/// callbacks such as `on_popup_dismiss`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
pub struct FunctionName(String);

impl FunctionName {
    #[must_use]
    pub(crate) fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    #[must_use]
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for FunctionName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
