use async_trait::async_trait;
use std::collections::HashMap;
use std::marker::PhantomData;
use tokio::sync::mpsc;
use tracing::{debug, error};
use zbus::Connection;

use crate::shared::dbus::domain::{
    BusType, DBusSubscription, DBusValue, Destination, Interface, Member, NameChangedStream,
    NameOwnerChanged, Path, PropertiesMap, PropertyChangedStream, PropertyName, SignalStream,
};
use crate::shared::dbus::ports::{DbusConnectionError, DbusConnectionPort};

pub struct Connected;
pub struct Disconnected;

pub struct ZbusConnectionAdapter<State = Disconnected> {
    session_conn: Option<Connection>,
    system_conn: Option<Connection>,
    _state: PhantomData<State>,
}

impl Default for ZbusConnectionAdapter<Disconnected> {
    fn default() -> Self {
        Self::new()
    }
}

impl ZbusConnectionAdapter<Disconnected> {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            session_conn: None,
            system_conn: None,
            _state: PhantomData,
        }
    }

    /// Connects to `DBus` session and system buses.
    ///
    /// # Errors
    ///
    /// Returns `DbusConnectionError` if connection to buses fails.
    pub async fn connect(
        mut self,
    ) -> Result<ZbusConnectionAdapter<Connected>, DbusConnectionError> {
        debug!("Connecting to DBus Session Bus...");
        match Connection::session().await {
            Ok(conn) => self.session_conn = Some(conn),
            Err(e) => error!("Failed to connect to Session Bus: {e}"),
        }

        debug!("Connecting to DBus System Bus...");
        match Connection::system().await {
            Ok(conn) => self.system_conn = Some(conn),
            Err(e) => error!("Failed to connect to System Bus: {e}"),
        }

        Ok(ZbusConnectionAdapter {
            session_conn: self.session_conn,
            system_conn: self.system_conn,
            _state: PhantomData,
        })
    }
}

impl ZbusConnectionAdapter<Connected> {
    fn get_conn(&self, bus: BusType) -> Result<&Connection, DbusConnectionError> {
        match bus {
            BusType::Session => self.session_conn.as_ref(),
            BusType::System => self.system_conn.as_ref(),
        }
        .ok_or(DbusConnectionError::NotInitialized(bus))
    }

    /// Convert a `zbus::zvariant::Value` to our domain `DBusValue`
    fn parse_value(val: &zbus::zvariant::Value<'_>) -> DBusValue {
        use zbus::zvariant::Value;
        match val {
            Value::Str(s) => DBusValue::String(s.as_str().to_string()),
            Value::I16(i) => DBusValue::Int(i64::from(*i)),
            Value::I32(i) => DBusValue::Int(i64::from(*i)),
            Value::I64(i) => DBusValue::Int(*i),
            Value::U16(u) => DBusValue::Int(i64::from(*u)),
            Value::U32(u) => DBusValue::Int(i64::from(*u)),
            Value::U64(u) => DBusValue::Int(i64::try_from(*u).unwrap_or(i64::MAX)),
            Value::F64(f) => DBusValue::Float(*f),
            Value::Bool(b) => DBusValue::Bool(*b),
            Value::Array(a) => {
                let mut items = Vec::new();
                for item in a.iter() {
                    items.push(Self::parse_value(item));
                }
                DBusValue::Array(items)
            }
            Value::Dict(d) => {
                let mut map = HashMap::new();
                for (k, v) in d.iter() {
                    if let Value::Str(key_str) = k {
                        map.insert(key_str.as_str().to_string(), Self::parse_value(v));
                    }
                }
                DBusValue::Dict(map)
            }
            Value::Value(v) => Self::parse_value(v),
            _ => DBusValue::Null,
        }
    }
}

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
        Ok(Self::parse_value(&body))
    }

    async fn get_all_properties(
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
        let dict: std::collections::HashMap<String, zbus::zvariant::Value> =
            msg_body.deserialize().map_err(|e| {
                DbusConnectionError::MethodCallFailed(format!(
                    "Failed to deserialize properties: {e}"
                ))
            })?;

        let mut map = HashMap::new();
        for (k, v) in dict {
            map.insert(PropertyName::new(k), Self::parse_value(&v));
        }

        Ok(PropertiesMap::new(map))
    }

    async fn subscribe_properties_changed(
        &self,
        bus: BusType,
        sender: &Destination,
        path: &Path,
    ) -> Result<PropertyChangedStream, DbusConnectionError> {
        use futures::StreamExt;

        let conn = self.get_conn(bus)?.clone();

        // We use MatchRule to listen for PropertiesChanged from this sender and path
        let rule = zbus::MatchRule::builder()
            .msg_type(zbus::message::Type::Signal)
            .sender(sender.as_str())
            .map_err(|e| DbusConnectionError::SubscriptionFailed(format!("Invalid sender: {e}")))?
            .path(path.as_str())
            .map_err(|e| DbusConnectionError::SubscriptionFailed(format!("Invalid path: {e}")))?
            .interface("org.freedesktop.DBus.Properties")
            .map_err(|e| {
                DbusConnectionError::SubscriptionFailed(format!("Invalid interface: {e}"))
            })?
            .member("PropertiesChanged")
            .map_err(|e| DbusConnectionError::SubscriptionFailed(format!("Invalid member: {e}")))?
            .build();

        let mut stream = zbus::MessageStream::for_match_rule(rule, &conn, None)
            .await
            .map_err(|e| DbusConnectionError::SubscriptionFailed(e.to_string()))?;

        let (tx, rx) = mpsc::unbounded_channel();

        tokio::spawn(async move {
            while let Some(msg_result) = stream.next().await {
                if let Ok(msg) = msg_result {
                    // PropertiesChanged signature: s a{sv} as (interface_name, changed_properties, invalidated_properties)
                    if let Ok((iface, changed, _invalidated)) = msg.body().deserialize::<(
                        String,
                        std::collections::HashMap<String, zbus::zvariant::Value>,
                        Vec<String>,
                    )>() {
                        let mut map = HashMap::new();
                        for (k, v) in changed {
                            map.insert(PropertyName::new(k), Self::parse_value(&v));
                        }
                        let props = PropertiesMap::new(map);
                        if tx.send((Interface::new(iface), props)).is_err() {
                            break;
                        }
                    }
                }
            }
        });

        Ok(Box::pin(
            tokio_stream::wrappers::UnboundedReceiverStream::new(rx),
        ))
    }

    async fn list_names(&self, bus: BusType) -> Result<Vec<Destination>, DbusConnectionError> {
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

    async fn subscribe_name_changes(
        &self,
        bus: BusType,
    ) -> Result<NameChangedStream, DbusConnectionError> {
        use futures::StreamExt;

        let conn = self.get_conn(bus)?.clone();

        let rule = zbus::MatchRule::builder()
            .msg_type(zbus::message::Type::Signal)
            .sender("org.freedesktop.DBus")
            .map_err(|e| DbusConnectionError::SubscriptionFailed(e.to_string()))?
            .path("/org/freedesktop/DBus")
            .map_err(|e| DbusConnectionError::SubscriptionFailed(e.to_string()))?
            .interface("org.freedesktop.DBus")
            .map_err(|e| DbusConnectionError::SubscriptionFailed(e.to_string()))?
            .member("NameOwnerChanged")
            .map_err(|e| DbusConnectionError::SubscriptionFailed(e.to_string()))?
            .build();

        let mut stream = zbus::MessageStream::for_match_rule(rule, &conn, None)
            .await
            .map_err(|e| DbusConnectionError::SubscriptionFailed(e.to_string()))?;

        let (tx, rx) = mpsc::unbounded_channel();

        tokio::spawn(async move {
            while let Some(msg_result) = stream.next().await {
                if let Ok(msg) = msg_result {
                    // NameOwnerChanged signature: s s s (name, old_owner, new_owner)
                    if let Ok((name, old_owner, new_owner)) =
                        msg.body().deserialize::<(String, String, String)>()
                    {
                        let old_opt = if old_owner.is_empty() {
                            None
                        } else {
                            Some(Destination::new(old_owner))
                        };
                        let new_opt = if new_owner.is_empty() {
                            None
                        } else {
                            Some(Destination::new(new_owner))
                        };
                        let event = NameOwnerChanged::new(Destination::new(name), old_opt, new_opt);

                        if tx.send(event).is_err() {
                            break;
                        }
                    }
                }
            }
        });

        Ok(Box::pin(
            tokio_stream::wrappers::UnboundedReceiverStream::new(rx),
        ))
    }

    async fn subscribe_signal(
        &self,
        sub: DBusSubscription,
    ) -> Result<SignalStream, DbusConnectionError> {
        use futures::StreamExt;

        let conn = self.get_conn(sub.bus())?.clone();

        let mut rule_builder = zbus::MatchRule::builder().msg_type(zbus::message::Type::Signal);

        if let Some(dest) = sub.destination() {
            rule_builder = rule_builder
                .sender(dest.as_str())
                .map_err(|e| DbusConnectionError::SubscriptionFailed(e.to_string()))?;
        }
        if let Some(path) = sub.path() {
            rule_builder = rule_builder
                .path(path.as_str())
                .map_err(|e| DbusConnectionError::SubscriptionFailed(e.to_string()))?;
        }
        if let Some(iface) = sub.interface() {
            rule_builder = rule_builder
                .interface(iface.as_str())
                .map_err(|e| DbusConnectionError::SubscriptionFailed(e.to_string()))?;
        }
        if let Some(member) = sub.member() {
            rule_builder = rule_builder
                .member(member.as_str())
                .map_err(|e| DbusConnectionError::SubscriptionFailed(e.to_string()))?;
        }

        let rule = rule_builder.build();
        let mut stream = zbus::MessageStream::for_match_rule(rule, &conn, None)
            .await
            .map_err(|e| DbusConnectionError::SubscriptionFailed(e.to_string()))?;

        let (tx, rx) = mpsc::unbounded_channel();

        tokio::spawn(async move {
            while let Some(msg_result) = stream.next().await {
                if let Ok(msg) = msg_result {
                    let header = msg.header();
                    let path = header
                        .path()
                        .map_or_else(|| Path::new(""), |p| Path::new(p.as_str()));
                    let member = header
                        .member()
                        .map_or_else(|| Member::new(""), |m| Member::new(m.as_str()));

                    if let Ok(body_val) = msg.body().deserialize::<zbus::zvariant::Value<'_>>() {
                        let parsed = Self::parse_value(&body_val);
                        if tx.send((path, member, parsed)).is_err() {
                            break;
                        }
                    }
                }
            }
        });

        Ok(Box::pin(
            tokio_stream::wrappers::UnboundedReceiverStream::new(rx),
        ))
    }
}
