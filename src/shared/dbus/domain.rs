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
    pub fn new(val: impl Into<String>) -> Self { Self(val.into()) }
    pub fn as_str(&self) -> &str { &self.0 }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Path(String);
impl Path {
    pub fn new(val: impl Into<String>) -> Self { Self(val.into()) }
    pub fn as_str(&self) -> &str { &self.0 }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Interface(String);
impl Interface {
    pub fn new(val: impl Into<String>) -> Self { Self(val.into()) }
    pub fn as_str(&self) -> &str { &self.0 }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Member(String);
impl Member {
    pub fn new(val: impl Into<String>) -> Self { Self(val.into()) }
    pub fn as_str(&self) -> &str { &self.0 }
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
    pub fn new(
        bus: BusType,
        destination: Option<Destination>,
        path: Option<Path>,
        interface: Option<Interface>,
        member: Option<Member>,
    ) -> Self {
        Self { bus, destination, path, interface, member }
    }

    pub fn bus(&self) -> BusType { self.bus }
    pub fn destination(&self) -> Option<&Destination> { self.destination.as_ref() }
    pub fn path(&self) -> Option<&Path> { self.path.as_ref() }
    pub fn interface(&self) -> Option<&Interface> { self.interface.as_ref() }
    pub fn member(&self) -> Option<&Member> { self.member.as_ref() }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DBusValue {
    String(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    Array(Vec<DBusValue>),
    Dict(HashMap<String, DBusValue>),
    Null,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct DBusState {
    properties: HashMap<String, DBusValue>,
}

impl DBusState {
    pub fn new(properties: HashMap<String, DBusValue>) -> Self {
        Self { properties }
    }

    pub fn properties(&self) -> &HashMap<String, DBusValue> {
        &self.properties
    }
}
