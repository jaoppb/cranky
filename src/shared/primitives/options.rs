use super::dynamic::DynamicValue;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Encapsulated options map passed to module instances
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ModuleOptions(HashMap<String, DynamicValue>);

impl ModuleOptions {
    #[must_use]
    pub(crate) const fn new(map: HashMap<String, DynamicValue>) -> Self {
        Self(map)
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    #[must_use]
    pub(crate) fn get(&self, key: &str) -> Option<&DynamicValue> {
        self.0.get(key)
    }
}
