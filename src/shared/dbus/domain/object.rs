//! Addresses of an object on a bus and of a member of one of its interfaces.

use super::types::{BusType, Destination, Interface, Member, Path};

/// One object on a message bus: the bus, the name of the connection that owns it, and its path.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BusObject {
    bus: BusType,
    destination: Destination,
    path: Path,
}

impl BusObject {
    /// Addresses the object at `path` owned by `destination` on `bus`.
    #[must_use]
    pub const fn new(bus: BusType, destination: Destination, path: Path) -> Self {
        Self {
            bus,
            destination,
            path,
        }
    }

    #[must_use]
    pub(crate) const fn bus(&self) -> BusType {
        self.bus
    }

    #[must_use]
    pub(crate) const fn destination(&self) -> &Destination {
        &self.destination
    }

    #[must_use]
    pub(crate) const fn path(&self) -> &Path {
        &self.path
    }
}

/// A member (here, a method) named together with the interface that declares it, as `DBus`
/// addresses it: `org.example.Interface.Method`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct QualifiedMember {
    interface: Interface,
    member: Member,
}

impl QualifiedMember {
    /// Names `member` of `interface`.
    #[must_use]
    pub const fn new(interface: Interface, member: Member) -> Self {
        Self { interface, member }
    }

    #[must_use]
    pub(crate) const fn interface(&self) -> &Interface {
        &self.interface
    }

    #[must_use]
    pub(crate) const fn member(&self) -> &Member {
        &self.member
    }
}
