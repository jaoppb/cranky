use async_trait::async_trait;
use crate::adapters::zbus::DBusPortError;
use crate::domain::dbus::{BusType, DBusSubscription, DBusValue};

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait DBusPort: Send + Sync {
    /// Initialize the connection to the DBus buses (session and system)
    async fn connect(&mut self) -> Result<(), DBusPortError>;
    
    /// Register a subscription dynamically from a module
    async fn subscribe(&mut self, sub: DBusSubscription) -> Result<(), DBusPortError>;
    
    /// Send an asynchronous command to the bus (e.g., Play/Pause for MPRIS)
    async fn call_method(
        &self,
        bus: BusType,
        destination: &str,
        path: &str,
        interface: &str,
        method: &str,
        args: Vec<DBusValue>,
    ) -> Result<(), DBusPortError>;
}
