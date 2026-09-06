use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::types::PropertyName;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DBusValue {
    String(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    Array(Vec<Self>),
    Dict(HashMap<String, Self>),
    Null,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct PropertiesMap(HashMap<PropertyName, DBusValue>);

impl PropertiesMap {
    #[must_use]
    pub const fn new(inner: HashMap<PropertyName, DBusValue>) -> Self {
        Self(inner)
    }

    #[must_use]
    pub fn get(&self, name: &PropertyName) -> Option<&DBusValue> {
        self.0.get(name)
    }

    #[must_use]
    pub fn iter(&self) -> std::collections::hash_map::Iter<'_, PropertyName, DBusValue> {
        self.0.iter()
    }
}

impl<'a> IntoIterator for &'a PropertiesMap {
    type Item = (&'a PropertyName, &'a DBusValue);
    type IntoIter = std::collections::hash_map::Iter<'a, PropertyName, DBusValue>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct DBusState {
    properties: HashMap<String, DBusValue>,
}

impl DBusState {
    #[must_use]
    pub const fn new(properties: HashMap<String, DBusValue>) -> Self {
        Self { properties }
    }

    #[must_use]
    pub const fn properties(&self) -> &HashMap<String, DBusValue> {
        &self.properties
    }
}
