//! The match rules that select the signals each subscription listens to.

use crate::shared::dbus::domain::{BusObject, DBusSubscription};
use crate::shared::dbus::ports::DbusConnectionError;

/// Matches the `PropertiesChanged` signals `object` sends.
pub(super) fn properties_changed(
    object: &BusObject,
) -> Result<zbus::MatchRule<'_>, DbusConnectionError> {
    let sender =
        zbus::names::BusName::try_from(object.destination().as_str()).map_err(|error| {
            DbusConnectionError::SubscriptionFailed(format!("Invalid sender: {error}"))
        })?;
    let path = zbus::ObjectPath::try_from(object.path().as_str()).map_err(|error| {
        DbusConnectionError::SubscriptionFailed(format!("Invalid path: {error}"))
    })?;
    zbus::MatchRule::builder()
        .msg_type(zbus::message::Type::Signal)
        .sender(sender)
        .path(path)
        .interface("org.freedesktop.DBus.Properties")
        .member("PropertiesChanged")
        .build()
        .map_err(|error| DbusConnectionError::SubscriptionFailed(error.to_string()))
}

/// Matches the bus daemon's `NameOwnerChanged` signals.
pub(super) fn name_owner_changed() -> Result<zbus::MatchRule<'static>, DbusConnectionError> {
    zbus::MatchRule::builder()
        .msg_type(zbus::message::Type::Signal)
        .sender("org.freedesktop.DBus")
        .path("/org/freedesktop/DBus")
        .interface("org.freedesktop.DBus")
        .member("NameOwnerChanged")
        .build()
        .map_err(|error| DbusConnectionError::SubscriptionFailed(error.to_string()))
}

/// Matches the signals `sub` names, narrowed by each of its parts that is set.
pub(super) fn signal(sub: &DBusSubscription) -> Result<zbus::MatchRule<'_>, DbusConnectionError> {
    let mut rule_builder = zbus::MatchRule::builder().msg_type(zbus::message::Type::Signal);

    if let Some(dest) = sub.destination() {
        rule_builder = rule_builder.sender(dest.as_str());
    }
    if let Some(path) = sub.path() {
        rule_builder = rule_builder.path(path.as_str());
    }
    if let Some(iface) = sub.interface() {
        rule_builder = rule_builder.interface(iface.as_str());
    }
    if let Some(member) = sub.member() {
        rule_builder = rule_builder.member(member.as_str());
    }

    rule_builder
        .build()
        .map_err(|error| DbusConnectionError::SubscriptionFailed(error.to_string()))
}
