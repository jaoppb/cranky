use crate::adapters::zbus::DBusPortError;
use crate::domain::dbus::DBusSubscription;
use async_trait::async_trait;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait DBusPort: Send + Sync {
    /// Initialize the connection to the DBus buses (session and system)
    async fn connect(&mut self) -> Result<(), DBusPortError>;

    /// Register a subscription dynamically from a module
    async fn subscribe(&mut self, sub: DBusSubscription) -> Result<(), DBusPortError>;
}
