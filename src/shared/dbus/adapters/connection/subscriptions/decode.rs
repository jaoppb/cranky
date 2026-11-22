//! Turns the messages a subscription receives into the items its stream yields.

use std::collections::HashMap;

use crate::shared::dbus::domain::{
    DBusValue, Destination, Interface, Member, NameOwnerChanged, Path, PropertiesMap, PropertyName,
};

use super::super::parser::parse_value;

/// The interface and changed properties of a `PropertiesChanged` signal, or `None` when its body
/// is not one.
pub(super) fn properties_changed(msg: &zbus::Message) -> Option<(Interface, PropertiesMap)> {
    let body = msg.body();
    let (iface, changed, _invalidated) = body
        .deserialize::<(String, HashMap<String, zbus::Variant<'_>>, Vec<String>)>()
        .ok()?;
    let map = changed
        .into_iter()
        .map(|(key, value)| (PropertyName::new(key), parse_value(&value)))
        .collect();
    Some((Interface::new(iface), PropertiesMap::new(map)))
}

/// The name and its old and new owners from a `NameOwnerChanged` signal, or `None` when its body
/// is not one.
pub(super) fn name_owner_changed(msg: &zbus::Message) -> Option<NameOwnerChanged> {
    let (name, old_owner, new_owner) = msg.body().deserialize::<(String, String, String)>().ok()?;
    let event = NameOwnerChanged::new(Destination::new(name), owner(old_owner), owner(new_owner));
    Some(event)
}

/// The owner `name` stands for, or `None` for the empty name that means no owner.
fn owner(name: String) -> Option<Destination> {
    if name.is_empty() {
        None
    } else {
        Some(Destination::new(name))
    }
}

/// The path, member and parsed variant body of a signal, or `None` when its body is not a
/// variant.
pub(super) fn signal(msg: &zbus::Message) -> Option<(Path, Member, DBusValue)> {
    let header = msg.header();
    let path = header.path().map_or_else(
        || Path::new(""),
        |object_path| Path::new(object_path.as_str()),
    );
    let member = header.member().map_or_else(
        || Member::new(""),
        |member_name| Member::new(member_name.as_str()),
    );
    let body = msg.body();
    let body_val = body.deserialize::<zbus::Variant<'_>>().ok()?;
    Some((path, member, parse_value(&body_val)))
}
