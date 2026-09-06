use std::collections::HashMap;

use crate::shared::dbus::domain::{
    BusType, DBusValue, Destination, Interface, Member, Path, PropertiesMap, PropertyName,
};
use crate::shared::dbus::ports::DbusConnectionError;

use super::adapter::{Connected, ZbusConnectionAdapter};
use super::parser::parse_value;

impl ZbusConnectionAdapter<Connected> {
    pub(crate) async fn call_method_impl(
        &self,
        bus: BusType,
        destination: &Destination,
        path: &Path,
        interface: &Interface,
        method: &Member,
    ) -> Result<DBusValue, DbusConnectionError> {
        let conn = self.get_conn(bus)?;
        let msg = conn
            .call_method(
                Some(destination.as_str()),
                path.as_str(),
                Some(interface.as_str()),
                method.as_str(),
                &(),
            )
            .await
            .map_err(|e| DbusConnectionError::MethodCallFailed(e.to_string()))?;
        let msg_body = msg.body();
        let body: zbus::zvariant::Value = msg_body
            .deserialize()
            .map_err(|e| DbusConnectionError::MethodCallFailed(e.to_string()))?;
        Ok(parse_value(&body))
    }

    pub(crate) async fn get_all_properties_impl(
        &self,
        bus: BusType,
        destination: &Destination,
        path: &Path,
        interface: &Interface,
    ) -> Result<PropertiesMap, DbusConnectionError> {
        let conn = self.get_conn(bus)?;
        let msg = conn
            .call_method(
                Some(destination.as_str()),
                path.as_str(),
                Some("org.freedesktop.DBus.Properties"),
                "GetAll",
                &(interface.as_str(),),
            )
            .await
            .map_err(|e| DbusConnectionError::MethodCallFailed(e.to_string()))?;

        let msg_body = msg.body();
        let dict: HashMap<String, zbus::zvariant::Value> =
            msg_body.deserialize().map_err(|e| {
                DbusConnectionError::MethodCallFailed(format!(
                    "Failed to deserialize properties: {e}"
                ))
            })?;

        let mut map = HashMap::new();
        for (k, v) in dict {
            map.insert(PropertyName::new(k), parse_value(&v));
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
            .map_err(|e| DbusConnectionError::MethodCallFailed(e.to_string()))?;

        let names: Vec<String> = msg.body().deserialize().map_err(|e| {
            DbusConnectionError::MethodCallFailed(format!("Failed to deserialize names: {e}"))
        })?;

        Ok(names.into_iter().map(Destination::new).collect())
    }
}
