use std::collections::HashMap;
use tokio::sync::mpsc;
use tokio_stream::StreamExt;

use crate::shared::dbus::domain::{
    BusType, DBusSubscription, Destination, Interface, Member, NameChangedStream, NameOwnerChanged,
    Path, PropertiesMap, PropertyChangedStream, PropertyName, SignalStream,
};
use crate::shared::dbus::ports::DbusConnectionError;

use super::adapter::{Connected, ZbusConnectionAdapter};
use super::parser::parse_value;

impl ZbusConnectionAdapter<Connected> {
    pub(crate) async fn subscribe_properties_changed_impl(
        &self,
        bus: BusType,
        sender: &Destination,
        path: &Path,
    ) -> Result<PropertyChangedStream, DbusConnectionError> {
        let conn = self.get_conn(bus)?.clone();

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
                if let Ok(msg) = msg_result
                    && let Ok((iface, changed, _invalidated)) = msg.body().deserialize::<(
                        String,
                        HashMap<String, zbus::zvariant::Value>,
                        Vec<String>,
                    )>()
                {
                    let mut map = HashMap::new();
                    for (k, v) in changed {
                        map.insert(PropertyName::new(k), parse_value(&v));
                    }
                    let props = PropertiesMap::new(map);
                    if tx.send((Interface::new(iface), props)).is_err() {
                        break;
                    }
                }
            }
        });

        Ok(Box::pin(
            tokio_stream::wrappers::UnboundedReceiverStream::new(rx),
        ))
    }

    pub(crate) async fn subscribe_name_changes_impl(
        &self,
        bus: BusType,
    ) -> Result<NameChangedStream, DbusConnectionError> {
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
                if let Ok(msg) = msg_result
                    && let Ok((name, old_owner, new_owner)) =
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
        });

        Ok(Box::pin(
            tokio_stream::wrappers::UnboundedReceiverStream::new(rx),
        ))
    }

    pub(crate) async fn subscribe_signal_impl(
        &self,
        sub: DBusSubscription,
    ) -> Result<SignalStream, DbusConnectionError> {
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
                        let parsed = parse_value(&body_val);
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
