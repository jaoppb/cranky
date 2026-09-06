use std::pin::Pin;

use futures::Stream;
use serde::{Deserialize, Serialize};

use super::types::{BusType, Destination, Interface, Member, Path};
use super::values::{DBusValue, PropertiesMap};

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
