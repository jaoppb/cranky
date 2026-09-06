use std::collections::HashMap;

use crate::shared::dbus::domain::DBusValue;

pub(crate) fn parse_value(val: &zbus::zvariant::Value<'_>) -> DBusValue {
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
                items.push(parse_value(item));
            }
            DBusValue::Array(items)
        }
        Value::Dict(d) => {
            let mut map = HashMap::new();
            for (k, v) in d.iter() {
                if let Value::Str(key_str) = k {
                    map.insert(key_str.as_str().to_string(), parse_value(v));
                }
            }
            DBusValue::Dict(map)
        }
        Value::Value(v) => parse_value(v),
        _ => DBusValue::Null,
    }
}
