use std::collections::HashMap;

use crate::shared::dbus::domain::DBusValue;

pub(super) fn parse_value(val: &zbus::Variant<'_>) -> DBusValue {
    use zbus::Variant;
    match val {
        Variant::Str(text) => DBusValue::String(text.as_str().to_string()),
        Variant::I16(integer) => DBusValue::Int(i64::from(*integer)),
        Variant::I32(integer) => DBusValue::Int(i64::from(*integer)),
        Variant::I64(integer) => DBusValue::Int(*integer),
        Variant::U16(unsigned) => DBusValue::Int(i64::from(*unsigned)),
        Variant::U32(unsigned) => DBusValue::Int(i64::from(*unsigned)),
        Variant::U64(unsigned) => DBusValue::Int(i64::try_from(*unsigned).unwrap_or(i64::MAX)),
        Variant::F64(float) => DBusValue::Float(*float),
        Variant::Bool(boolean) => DBusValue::Bool(*boolean),
        Variant::Array(array) => {
            let mut items = Vec::new();
            for item in array.iter() {
                items.push(parse_value(item));
            }
            DBusValue::Array(items)
        }
        Variant::Dict(dict) => {
            let mut map = HashMap::new();
            for (key, value) in dict.iter() {
                if let Variant::Str(key_str) = key {
                    map.insert(key_str.as_str().to_string(), parse_value(value));
                }
            }
            DBusValue::Dict(map)
        }
        Variant::Variant(inner) => parse_value(inner),
        _ => DBusValue::Null,
    }
}
