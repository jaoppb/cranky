use crate::shared::dbus::domain::DBusSubscription;
use async_trait::async_trait;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DBusPortError {
    #[error("Failed to subscribe to DBus signal: {0}")]
    Subscription(String),
}

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait DBusPort: Send + Sync {
    /// Initialize the connection to the DBus buses (session and system)
    async fn connect(&mut self) -> Result<(), DBusPortError>;

    /// Register a subscription dynamically from a module
    async fn subscribe(&mut self, sub: DBusSubscription) -> Result<(), DBusPortError>;
}
