use crate::shared::dbus::domain::{
    BusType, DBusSubscription, DBusValue, Destination, Interface, Member, NameChangedStream, Path,
    PropertiesMap, PropertyChangedStream, SignalStream,
};
use async_trait::async_trait;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DbusConnectionError {
    #[error("Connection not initialized for bus: {0:?}")]
    NotInitialized(BusType),
    #[error("Method call failed: {0}")]
    MethodCallFailed(String),
    #[error("Subscription failed: {0}")]
    SubscriptionFailed(String),
    #[error("Property read failed: {0}")]
    PropertyReadFailed(String),
}

/// High-level `DBus` bus operations port.
/// Consumers use this to interact with `DBus` without knowing the underlying implementation.
/// Implemented by a connected adapter.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait DbusConnectionPort: Send + Sync {
    /// Call a method on a `DBus` interface and return the result as a `DBusValue`.
    ///
    /// # Errors
    ///
    /// Returns `DbusConnectionError` if the method call or serialization fails.
    async fn call_method(
        &self,
        bus: BusType,
        destination: &Destination,
        path: &Path,
        interface: &Interface,
        method: &Member,
    ) -> Result<DBusValue, DbusConnectionError>;

    /// Get all properties from a `DBus` interface.
    ///
    /// # Errors
    ///
    /// Returns `DbusConnectionError` if getting properties fails.
    async fn get_all_properties(
        &self,
        bus: BusType,
        destination: &Destination,
        path: &Path,
        interface: &Interface,
    ) -> Result<PropertiesMap, DbusConnectionError>;

    /// Subscribe to `PropertiesChanged` signals on a specific path.
    /// Returns a stream of `(interface_name, changed_properties)` tuples.
    ///
    /// # Errors
    ///
    /// Returns `DbusConnectionError` if creating the match rule or subscription fails.
    async fn subscribe_properties_changed(
        &self,
        bus: BusType,
        sender: &Destination,
        path: &Path,
    ) -> Result<PropertyChangedStream, DbusConnectionError>;

    /// List all currently owned bus names.
    ///
    /// # Errors
    ///
    /// Returns `DbusConnectionError` if calling `ListNames` fails.
    async fn list_names(&self, bus: BusType) -> Result<Vec<Destination>, DbusConnectionError>;

    /// Subscribe to `NameOwnerChanged` signals for tracking bus name appearance/disappearance.
    ///
    /// # Errors
    ///
    /// Returns `DbusConnectionError` if subscribing to name changes fails.
    async fn subscribe_name_changes(
        &self,
        bus: BusType,
    ) -> Result<NameChangedStream, DbusConnectionError>;

    /// Subscribe to an arbitrary `DBus` signal.
    ///
    /// # Errors
    ///
    /// Returns `DbusConnectionError` if building the match rule or subscribing fails.
    async fn subscribe_signal(
        &self,
        sub: DBusSubscription,
    ) -> Result<SignalStream, DbusConnectionError>;
}
