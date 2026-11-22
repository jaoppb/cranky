use std::collections::HashMap;

use crate::shared::dbus::domain::{
    BusObject, BusType, DBusValue, Destination, Interface, PropertiesMap, PropertyName,
    QualifiedMember,
};
use crate::shared::dbus::ports::DbusConnectionError;

use super::adapter::{Connected, ZbusConnectionAdapter};
use super::parser::parse_value;

impl ZbusConnectionAdapter<Connected> {
    pub(crate) async fn call_method_impl(
        &self,
        object: &BusObject,
        method: &QualifiedMember,
    ) -> Result<DBusValue, DbusConnectionError> {
        let conn = self.get_conn(object.bus())?;
        let msg = conn
            .call_method(
                Some(object.destination().as_str()),
                object.path().as_str(),
                Some(method.interface().as_str()),
                method.member().as_str(),
                &(),
            )
            .await
            .map_err(|error| DbusConnectionError::MethodCallFailed(error.to_string()))?;
        let msg_body = msg.body();
        let body: zbus::Variant<'_> = msg_body
            .deserialize()
            .map_err(|error| DbusConnectionError::MethodCallFailed(error.to_string()))?;
        Ok(parse_value(&body))
    }

    pub(crate) async fn get_all_properties_impl(
        &self,
        object: &BusObject,
        interface: &Interface,
    ) -> Result<PropertiesMap, DbusConnectionError> {
        let conn = self.get_conn(object.bus())?;
        let msg = conn
            .call_method(
                Some(object.destination().as_str()),
                object.path().as_str(),
                Some("org.freedesktop.DBus.Properties"),
                "GetAll",
                &(interface.as_str(),),
            )
            .await
            .map_err(|error| DbusConnectionError::MethodCallFailed(error.to_string()))?;

        let msg_body = msg.body();
        let dict: HashMap<String, zbus::Variant<'_>> = msg_body.deserialize().map_err(|error| {
            DbusConnectionError::MethodCallFailed(format!(
                "Failed to deserialize properties: {error}"
            ))
        })?;

        let mut map = HashMap::new();
        for (key, value) in dict {
            map.insert(PropertyName::new(key), parse_value(&value));
        }

        Ok(PropertiesMap::new(map))
    }

    pub(crate) async fn list_names_impl(
        &self,
        bus: BusType,
    ) -> Result<Vec<Destination>, DbusConnectionError> {
        let conn = self.get_conn(bus)?;
        let msg = conn
            .call_method(
                Some("org.freedesktop.DBus"),
                "/org/freedesktop/DBus",
                Some("org.freedesktop.DBus"),
                "ListNames",
                &(),
            )
            .await
            .map_err(|error| DbusConnectionError::MethodCallFailed(error.to_string()))?;

        let names: Vec<String> = msg.body().deserialize().map_err(|error| {
            DbusConnectionError::MethodCallFailed(format!("Failed to deserialize names: {error}"))
        })?;

        Ok(names.into_iter().map(Destination::new).collect())
    }
}
