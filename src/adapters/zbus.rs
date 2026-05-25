use async_trait::async_trait;
use zbus::{Connection, MatchRule, Message, MessageStream};
use zbus::zvariant::Value;
use std::collections::HashMap;
use tokio::sync::watch;
use tokio_stream::StreamExt;
use tracing::{debug, error, info};

use crate::domain::dbus::{BusType, DBusState, DBusSubscription, DBusValue};
use crate::domain::errors::PortError;
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
            _ => DBusValue::Null,
        }
    }

    async fn handle_message(&self, msg: &Message) {
        let mut state = self.dbus_tx.borrow().clone();
        let header = msg.header();
        
        let path = match header.path() {
            Some(p) => {
                let p: &zbus::zvariant::ObjectPath<'_> = &*p;
                p.as_str().to_string()
            },
            None => String::new(),
        };
        let member = match header.member() {
            Some(m) => {
                let m: &zbus::names::MemberName<'_> = &*m;
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
    async fn connect(&mut self) -> Result<(), PortError> {
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

    async fn subscribe(&mut self, sub: DBusSubscription) -> Result<(), PortError> {
        let conn = match sub.bus {
            BusType::Session => self.session_conn.clone(),
            BusType::System => self.system_conn.clone(),
        };
        
        let Some(conn) = conn else {
            return Err(PortError::DBusError { reason: "DBus connection not initialized".into() });
        };

        let mut rule_builder = MatchRule::builder().msg_type(zbus::message::Type::Signal);
        
        if let Some(ref dest) = sub.destination {
            // Using sender instead of destination for signals
            rule_builder = rule_builder.sender(dest.clone()).map_err(|e| PortError::DBusError { reason: e.to_string() })?;
        }
        if let Some(ref path) = sub.path {
            rule_builder = rule_builder.path(path.clone()).map_err(|e| PortError::DBusError { reason: e.to_string() })?;
        }
        if let Some(ref iface) = sub.interface {
            rule_builder = rule_builder.interface(iface.clone()).map_err(|e| PortError::DBusError { reason: e.to_string() })?;
        }
        if let Some(ref member) = sub.member {
            rule_builder = rule_builder.member(member.clone()).map_err(|e| PortError::DBusError { reason: e.to_string() })?;
        }

        let rule = rule_builder.build();
        let mut stream = MessageStream::for_match_rule(rule, &conn, None)
            .await
            .map_err(|e| PortError::DBusError { reason: format!("Failed to create MessageStream: {}", e) })?;

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
                        let p: &zbus::zvariant::ObjectPath<'_> = &*p;
                        p.as_str().to_string()
                    },
                    None => String::new(),
                };
                let member = match header.member() {
                    Some(m) => {
                        let m: &zbus::names::MemberName<'_> = &*m;
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
        bus: BusType,
        destination: &str,
        path: &str,
        interface: &str,
        method: &str,
        args: Vec<DBusValue>,
    ) -> Result<(), PortError> {
        // Method calling will be implemented in a future iteration
        Ok(())
    }
}
