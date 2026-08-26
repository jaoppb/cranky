use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
pub struct DBusSubscription {
    bus: BusType,
    destination: Option<Destination>,
    path: Option<Path>,
    interface: Option<Interface>,
    member: Option<Member>,
}

impl DBusSubscription {
    #[must_use]
    pub const fn new(
        bus: BusType,
        destination: Option<Destination>,
        path: Option<Path>,
        interface: Option<Interface>,
        member: Option<Member>,
    ) -> Self {
        Self {
            bus,
            destination,
            path,
            interface,
            member,
        }
    }

    #[must_use]
    pub const fn bus(&self) -> BusType {
        self.bus
    }
    #[must_use]
    pub const fn destination(&self) -> Option<&Destination> {
        self.destination.as_ref()
    }
    #[must_use]
    pub const fn path(&self) -> Option<&Path> {
        self.path.as_ref()
    }
    #[must_use]
    pub const fn interface(&self) -> Option<&Interface> {
        self.interface.as_ref()
    }
    #[must_use]
    pub const fn member(&self) -> Option<&Member> {
        self.member.as_ref()
    }
}

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

use futures::Stream;
use std::pin::Pin;

/// A stream of property change signals from `DBus`.
/// Consumers await this for updates without knowing the underlying implementation.
pub type PropertyChangedStream =
    Pin<Box<dyn Stream<Item = (Interface, PropertiesMap)> + Send + Sync>>;

/// A stream of bus name ownership changes.
pub type NameChangedStream = Pin<Box<dyn Stream<Item = NameOwnerChanged> + Send + Sync>>;

#[derive(Debug, Clone)]
pub struct NameOwnerChanged {
    name: Destination,
    old_owner: Option<Destination>,
    new_owner: Option<Destination>,
}

impl NameOwnerChanged {
    #[must_use]
    pub const fn new(
        name: Destination,
        old_owner: Option<Destination>,
        new_owner: Option<Destination>,
    ) -> Self {
        Self {
            name,
            old_owner,
            new_owner,
        }
    }
    #[must_use]
    pub const fn name(&self) -> &Destination {
        &self.name
    }
    #[must_use]
    pub const fn old_owner(&self) -> Option<&Destination> {
        self.old_owner.as_ref()
    }
    #[must_use]
    pub const fn new_owner(&self) -> Option<&Destination> {
        self.new_owner.as_ref()
    }
    #[must_use]
    pub const fn is_new(&self) -> bool {
        self.old_owner.is_none() && self.new_owner.is_some()
    }
    #[must_use]
    pub const fn is_gone(&self) -> bool {
        self.old_owner.is_some() && self.new_owner.is_none()
    }
}

pub type SignalStream = Pin<Box<dyn Stream<Item = (Path, Member, DBusValue)> + Send + Sync>>;

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
