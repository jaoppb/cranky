use super::dynamic::DynamicValue;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Encapsulated options map passed to module instances
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ModuleOptions(HashMap<String, DynamicValue>);

impl ModuleOptions {
    #[must_use]
    pub const fn new(map: HashMap<String, DynamicValue>) -> Self {
        Self(map)
    }

    #[must_use]
    pub const fn as_map(&self) -> &HashMap<String, DynamicValue> {
        &self.0
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    #[must_use]
    pub fn get(&self, key: &str) -> Option<&DynamicValue> {
        self.0.get(key)
    }
}
