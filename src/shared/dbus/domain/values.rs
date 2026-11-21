//! `DBus` values as scripts see them, and the property state published on the signal hub.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::types::PropertyName;

/// A `DBus` variant converted into the few shapes scripts can consume; serialises untagged, as the
/// bare value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DBusValue {
    /// A string, object path or signature.
    String(String),
    /// Any signed or unsigned integer, widened to `i64` (a `u64` above `i64::MAX` saturates).
    Int(i64),
    /// A `d` (IEEE 754 double) value, unchanged.
    Float(f64),
    /// A `b` value, unchanged.
    Bool(bool),
    /// An array, element by element.
    Array(Vec<Self>),
    /// A dictionary, with keys turned into strings.
    Dict(HashMap<String, Self>),
    /// A variant type the conversion does not support.
    Null,
}

/// The properties of one interface, from `GetAll` or one `PropertiesChanged` signal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct PropertiesMap(HashMap<PropertyName, DBusValue>);

impl PropertiesMap {
    #[must_use]
    pub(crate) const fn new(inner: HashMap<PropertyName, DBusValue>) -> Self {
        Self(inner)
    }

    /// The value of one property, or `None` when the interface did not report it.
    #[must_use]
    pub fn get(&self, name: &PropertyName) -> Option<&DBusValue> {
        self.0.get(name)
    }

    #[must_use]
    pub(crate) fn iter(&self) -> std::collections::hash_map::Iter<'_, PropertyName, DBusValue> {
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
pub(crate) struct DBusState {
    properties: HashMap<String, DBusValue>,
}

impl DBusState {
    #[must_use]
    pub(crate) const fn new(properties: HashMap<String, DBusValue>) -> Self {
        Self { properties }
    }

    #[must_use]
    pub(crate) const fn properties(&self) -> &HashMap<String, DBusValue> {
        &self.properties
    }
}
