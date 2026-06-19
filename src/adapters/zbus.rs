use async_trait::async_trait;
use zbus::{Connection, MatchRule, Message, MessageStream};
use zbus::zvariant::Value;
use std::collections::HashMap;
use tokio::sync::watch;
use tokio_stream::StreamExt;
use tracing::{error, info};

use crate::domain::dbus::{BusType, DBusState, DBusSubscription, DBusValue};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DBusPortError {
    #[error("DBus error: {reason}")]
    DBusError { reason: String },
}

use crate::ports::DBusPort;
use crate::domain::signals::SignalHub;

pub struct ZbusAdapter {
    session_conn: Option<Connection>,
    system_conn: Option<Connection>,
    dbus_tx: watch::Sender<DBusState>,
}

impl ZbusAdapter {
    pub fn new(hub: &SignalHub) -> Self {
        Self {
            session_conn: None,
            system_conn: None,
            dbus_tx: hub.dbus_tx(),
        }
    }

    fn parse_value(val: &Value<'_>) -> DBusValue {
        match val {
            Value::Str(s) => DBusValue::String(s.to_string()),
            Value::U8(n) => DBusValue::Int(*n as i64),
            Value::I16(n) => DBusValue::Int(*n as i64),
            Value::U16(n) => DBusValue::Int(*n as i64),
            Value::I32(n) => DBusValue::Int(*n as i64),
            Value::U32(n) => DBusValue::Int(*n as i64),
            Value::I64(n) => DBusValue::Int(*n),
            Value::U64(n) => DBusValue::Int(*n as i64),
            Value::F64(f) => DBusValue::Float(*f),
            Value::Bool(b) => DBusValue::Bool(*b),
            Value::Array(a) => {
                let mut vec = Vec::new();
                for item in a.iter() {
                    vec.push(Self::parse_value(item));
                }
                DBusValue::Array(vec)
            }
            Value::Dict(d) => {
                let mut map = HashMap::new();
                for (k, v) in d.iter() {
                    if let Value::Str(key_str) = k {
                        map.insert(key_str.to_string(), Self::parse_value(v));
                    }
                }
                DBusValue::Dict(map)
            }
            Value::Value(v) => Self::parse_value(v),
            _ => DBusValue::Null,
        }
    }

    async fn handle_message(&self, msg: &Message) {
        let mut state = self.dbus_tx.borrow().clone();
        let header = msg.header();
        
        let path = match header.path() {
            Some(p) => {
                let p: &zbus::zvariant::ObjectPath<'_> = p;
                p.as_str().to_string()
            },
            None => String::new(),
        };
        let member = match header.member() {
            Some(m) => {
                let m: &zbus::names::MemberName<'_> = m;
                m.as_str().to_string()
            },
            None => String::new(),
        };
        
        // Handle standard PropertiesChanged
        if member == "PropertiesChanged" {
            let body = msg.body();
            // signature: sa{sv}as
            if let Ok((iface, changed, _invalidated)) = body.deserialize::<(String, HashMap<String, Value<'_>>, Vec<String>)>() {
                for (k, v) in changed.iter() {
                    let parsed = Self::parse_value(v);
                    let prop_key = format!("{}.{}", iface, k);
                    state.properties.insert(prop_key, parsed);
                }
            }
        } else {
            // For other signals, maybe just store the first argument
            if let Ok(body_val) = msg.body().deserialize::<Value<'_>>() {
                let parsed = Self::parse_value(&body_val);
                let prop_key = format!("{}.{}", path, member);
                state.properties.insert(prop_key, parsed);
            }
        }
        
        let _ = self.dbus_tx.send(state);
    }
}

#[async_trait]
impl DBusPort for ZbusAdapter {
    async fn connect(&mut self) -> Result<(), DBusPortError> {
        info!("Connecting to DBus Session Bus...");
        match Connection::session().await {
            Ok(conn) => self.session_conn = Some(conn),
            Err(e) => error!("Failed to connect to Session Bus: {}", e),
        }
        
        info!("Connecting to DBus System Bus...");
        match Connection::system().await {
            Ok(conn) => self.system_conn = Some(conn),
            Err(e) => error!("Failed to connect to System Bus: {}", e),
        }
        
        Ok(())
    }

    async fn subscribe(&mut self, sub: DBusSubscription) -> Result<(), DBusPortError> {
        let conn = match sub.bus {
            BusType::Session => self.session_conn.clone(),
            BusType::System => self.system_conn.clone(),
        };
        
        let Some(conn) = conn else {
            return Err(DBusPortError::DBusError { reason: "DBus connection not initialized".into() });
        };

        let mut rule_builder = MatchRule::builder().msg_type(zbus::message::Type::Signal);
        
        if let Some(ref dest) = sub.destination {
            // Using sender instead of destination for signals
            rule_builder = rule_builder.sender(dest.clone()).map_err(|e| DBusPortError::DBusError { reason: e.to_string() })?;
        }
        if let Some(ref path) = sub.path {
            rule_builder = rule_builder.path(path.clone()).map_err(|e| DBusPortError::DBusError { reason: e.to_string() })?;
        }
        if let Some(ref iface) = sub.interface {
            rule_builder = rule_builder.interface(iface.clone()).map_err(|e| DBusPortError::DBusError { reason: e.to_string() })?;
        }
        if let Some(ref member) = sub.member {
            rule_builder = rule_builder.member(member.clone()).map_err(|e| DBusPortError::DBusError { reason: e.to_string() })?;
        }

        let rule = rule_builder.build();
        let mut stream = MessageStream::for_match_rule(rule, &conn, None)
            .await
            .map_err(|e| DBusPortError::DBusError { reason: format!("Failed to create MessageStream: {}", e) })?;

        info!("Subscribed to DBus match rule");

        let tx = self.dbus_tx.clone();
        
        tokio::spawn(async move {
            while let Some(Ok(msg)) = stream.next().await {
                // Parse message and update state
                // This is a simplified version, ideally we would share `handle_message` logic.
                let mut state = tx.borrow().clone();
                let header = msg.header();
                
                let path = match header.path() {
                    Some(p) => {
                        let p: &zbus::zvariant::ObjectPath<'_> = p;
                        p.as_str().to_string()
                    },
                    None => String::new(),
                };
                let member = match header.member() {
                    Some(m) => {
                        let m: &zbus::names::MemberName<'_> = m;
                        m.as_str().to_string()
                    },
                    None => String::new(),
                };
                
                if member == "PropertiesChanged" {
                    if let Ok((iface, changed, _invalidated)) = msg.body().deserialize::<(String, HashMap<String, Value<'_>>, Vec<String>)>() {
                        for (k, v) in changed.iter() {
                            let parsed = ZbusAdapter::parse_value(v);
                            let prop_key = format!("{}.{}", iface, k);
                            state.properties.insert(prop_key, parsed);
                        }
                    }
                } else {
                    if let Ok(body_val) = msg.body().deserialize::<Value<'_>>() {
                        let parsed = ZbusAdapter::parse_value(&body_val);
                        let prop_key = format!("{}.{}", path, member);
                        state.properties.insert(prop_key, parsed);
                    }
                }
                
                let _ = tx.send(state);
            }
        });

        Ok(())
    }

    async fn call_method(
        &self,
        _bus: BusType,
        _destination: &str,
        _path: &str,
        _interface: &str,
        _method: &str,
        _args: Vec<DBusValue>,
    ) -> Result<(), DBusPortError> {
        // Method calling will be implemented in a future iteration
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use zbus::zvariant::{Value, Array, Dict, Signature};

    #[test]
    fn test_parse_value_primitives() {
        assert_eq!(ZbusAdapter::parse_value(&Value::Str("hello".into())), DBusValue::String("hello".into()));
        assert_eq!(ZbusAdapter::parse_value(&Value::U8(255)), DBusValue::Int(255));
        assert_eq!(ZbusAdapter::parse_value(&Value::I16(-100)), DBusValue::Int(-100));
        assert_eq!(ZbusAdapter::parse_value(&Value::U16(100)), DBusValue::Int(100));
        assert_eq!(ZbusAdapter::parse_value(&Value::I32(-1000)), DBusValue::Int(-1000));
        assert_eq!(ZbusAdapter::parse_value(&Value::U32(1000)), DBusValue::Int(1000));
        assert_eq!(ZbusAdapter::parse_value(&Value::I64(-10000)), DBusValue::Int(-10000));
        assert_eq!(ZbusAdapter::parse_value(&Value::U64(10000)), DBusValue::Int(10000));
        assert_eq!(ZbusAdapter::parse_value(&Value::F64(3.5)), DBusValue::Float(3.5));
        assert_eq!(ZbusAdapter::parse_value(&Value::Bool(true)), DBusValue::Bool(true));
        assert_eq!(ZbusAdapter::parse_value(&Value::Bool(false)), DBusValue::Bool(false));
    }

    #[test]
    fn test_parse_value_array() {
        let arr = Array::from(vec![Value::I32(1), Value::I32(2)]);
        let val = Value::Array(arr);
        let parsed = ZbusAdapter::parse_value(&val);
        assert_eq!(parsed, DBusValue::Array(vec![DBusValue::Int(1), DBusValue::Int(2)]));
    }

    #[test]
    fn test_parse_value_dict() {
        // Dict::new requires type signatures.
        let key_sig = zbus::zvariant::Signature::try_from("s").unwrap();
        let val_sig = zbus::zvariant::Signature::try_from("i").unwrap();
        let mut dict = Dict::new(&key_sig, &val_sig);
        dict.append(Value::Str("key1".into()), Value::I32(42)).unwrap();
        let val = Value::Dict(dict);
        let parsed = ZbusAdapter::parse_value(&val);
        
        let mut expected_map = HashMap::new();
        expected_map.insert("key1".into(), DBusValue::Int(42));
        assert_eq!(parsed, DBusValue::Dict(expected_map));
    }

    #[test]
    fn test_parse_value_null_for_unsupported() {
        // Value::ObjectPath is an example of an unsupported type.
        let val = Value::ObjectPath(zbus::zvariant::ObjectPath::try_from("/").unwrap());
        assert_eq!(ZbusAdapter::parse_value(&val), DBusValue::Null);
    }

    #[tokio::test]
    async fn test_zbus_connect_and_methods() {
        let hub = SignalHub::new(crate::domain::config::Config::default());
        let mut adapter = ZbusAdapter::new(&hub);
        
        // This will either succeed or fail depending on if DBus is running.
        // But it exercises the code paths!
        let _ = adapter.connect().await;

        let sub = DBusSubscription {
            bus: BusType::Session,
            destination: Some("org.freedesktop.DBus".into()),
            path: Some("/org/freedesktop/DBus".into()),
            interface: Some("org.freedesktop.DBus".into()),
            member: Some("NameOwnerChanged".into()),
        };
        
        // Subscribe exercises MatchRule building
        let _ = adapter.subscribe(sub).await;
        
        // call_method is a stub, but exercises code path
        let _ = adapter.call_method(
            BusType::Session,
            "org.freedesktop.DBus",
            "/org/freedesktop/DBus",
            "org.freedesktop.DBus",
            "ListNames",
            vec![]
        ).await;
    }

    #[tokio::test]
    async fn test_handle_message_properties_changed() {
        let hub = SignalHub::new(crate::domain::config::Config::default());
        let adapter = ZbusAdapter::new(&hub);
        
        let mut changed_props: HashMap<String, Value<'_>> = HashMap::new();
        changed_props.insert("SomeInt".to_string(), Value::I32(42));
        
        let msg = zbus::Message::signal(
            "/",
            "org.test.Iface",
            "PropertiesChanged",
        )
        .unwrap()
        .build(&("org.test.Iface".to_string(), changed_props, Vec::<String>::new()))
        .unwrap();
        
        adapter.handle_message(&msg).await;
        
        let state = hub.dbus_tx().borrow().clone();
        assert_eq!(state.properties.get("org.test.Iface.SomeInt"), Some(&DBusValue::Int(42)));
    }

    #[tokio::test]
    async fn test_handle_message_other() {
        let hub = SignalHub::new(crate::domain::config::Config::default());
        let adapter = ZbusAdapter::new(&hub);
        
        let msg = zbus::Message::signal(
            "/org/test",
            "org.test.Iface",
            "SomeSignal",
        )
        .unwrap()
        .build(&(Value::Str("test".into())))
        .unwrap();
        
        adapter.handle_message(&msg).await;
        
        let state = hub.dbus_tx().borrow().clone();
        assert_eq!(state.properties.get("/org/test.SomeSignal"), Some(&DBusValue::String("test".into())));
    }
}
