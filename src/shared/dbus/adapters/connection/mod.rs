pub mod adapter;
pub mod methods;
pub mod parser;
pub mod subscriptions;

pub use adapter::{Connected, Disconnected, ZbusConnectionAdapter};

use async_trait::async_trait;

use crate::shared::dbus::domain::{
    BusType, DBusSubscription, DBusValue, Destination, Interface, Member, NameChangedStream, Path,
    PropertiesMap, PropertyChangedStream, SignalStream,
};
use crate::shared::dbus::ports::{DbusConnectionError, DbusConnectionPort};

#[async_trait]
impl DbusConnectionPort for ZbusConnectionAdapter<Connected> {
    async fn call_method(
        &self,
        bus: BusType,
        destination: &Destination,
        path: &Path,
        interface: &Interface,
        method: &Member,
    ) -> Result<DBusValue, DbusConnectionError> {
        self.call_method_impl(bus, destination, path, interface, method)
            .await
    }

    async fn get_all_properties(
        &self,
        bus: BusType,
        destination: &Destination,
        path: &Path,
        interface: &Interface,
    ) -> Result<PropertiesMap, DbusConnectionError> {
        self.get_all_properties_impl(bus, destination, path, interface)
            .await
    }

    async fn subscribe_properties_changed(
        &self,
        bus: BusType,
        sender: &Destination,
        path: &Path,
    ) -> Result<PropertyChangedStream, DbusConnectionError> {
        self.subscribe_properties_changed_impl(bus, sender, path)
            .await
    }

    async fn list_names(&self, bus: BusType) -> Result<Vec<Destination>, DbusConnectionError> {
        self.list_names_impl(bus).await
    }

    async fn subscribe_name_changes(
        &self,
        bus: BusType,
    ) -> Result<NameChangedStream, DbusConnectionError> {
        self.subscribe_name_changes_impl(bus).await
    }

    async fn subscribe_signal(
        &self,
        sub: DBusSubscription,
    ) -> Result<SignalStream, DbusConnectionError> {
        self.subscribe_signal_impl(sub).await
    }
}
