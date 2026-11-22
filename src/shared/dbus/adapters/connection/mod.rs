//! The zbus implementation of `DbusConnectionPort`, with conversion of zbus variants into
//! `DBusValue`.

mod adapter;
mod methods;
mod parser;
mod subscriptions;

pub use adapter::{Connected, Disconnected, ZbusConnectionAdapter};

use async_trait::async_trait;

use crate::shared::dbus::domain::{
    BusObject, BusType, DBusSubscription, DBusValue, Destination, Interface, NameChangedStream,
    PropertiesMap, PropertyChangedStream, QualifiedMember, SignalStream,
};
use crate::shared::dbus::ports::{DbusConnectionError, DbusConnectionPort};

#[async_trait]
impl DbusConnectionPort for ZbusConnectionAdapter<Connected> {
    async fn call_method(
        &self,
        object: &BusObject,
        method: &QualifiedMember,
    ) -> Result<DBusValue, DbusConnectionError> {
        self.call_method_impl(object, method).await
    }

    async fn get_all_properties(
        &self,
        object: &BusObject,
        interface: &Interface,
    ) -> Result<PropertiesMap, DbusConnectionError> {
        self.get_all_properties_impl(object, interface).await
    }

    async fn subscribe_properties_changed(
        &self,
        object: &BusObject,
    ) -> Result<PropertyChangedStream, DbusConnectionError> {
        self.subscribe_properties_changed_impl(object).await
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
