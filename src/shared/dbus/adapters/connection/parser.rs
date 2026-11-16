use std::collections::HashMap;

use crate::shared::dbus::domain::DBusValue;

pub(crate) fn parse_value(val: &zbus::Variant<'_>) -> DBusValue {
    use zbus::Variant;
    match val {
        Variant::Str(s) => DBusValue::String(s.as_str().to_string()),
        Variant::I16(i) => DBusValue::Int(i64::from(*i)),
        Variant::I32(i) => DBusValue::Int(i64::from(*i)),
        Variant::I64(i) => DBusValue::Int(*i),
        Variant::U16(u) => DBusValue::Int(i64::from(*u)),
        Variant::U32(u) => DBusValue::Int(i64::from(*u)),
        Variant::U64(u) => DBusValue::Int(i64::try_from(*u).unwrap_or(i64::MAX)),
        Variant::F64(f) => DBusValue::Float(*f),
        Variant::Bool(b) => DBusValue::Bool(*b),
        Variant::Array(a) => {
            let mut items = Vec::new();
            for item in a.iter() {
                items.push(parse_value(item));
            }
            DBusValue::Array(items)
        }
        Variant::Dict(d) => {
            let mut map = HashMap::new();
            for (k, v) in d.iter() {
                if let Variant::Str(key_str) = k {
                    map.insert(key_str.as_str().to_string(), parse_value(v));
                }
            }
            DBusValue::Dict(map)
        }
        Variant::Variant(v) => parse_value(v),
        _ => DBusValue::Null,
    }
}
