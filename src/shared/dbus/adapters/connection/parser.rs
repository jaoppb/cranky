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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use zbus::Variant;

    use super::parse_value;
    use crate::shared::dbus::domain::DBusValue;

    #[test]
    fn every_integer_width_becomes_an_int() {
        let parsed = [
            Variant::from(-3_i16),
            Variant::from(-4_i32),
            Variant::from(-5_i64),
            Variant::from(6_u16),
            Variant::from(7_u32),
            Variant::from(8_u64),
        ]
        .iter()
        .map(parse_value)
        .collect::<Vec<_>>();
        assert_eq!(parsed, [-3, -4, -5, 6, 7, 8].map(DBusValue::Int).to_vec());
    }

    #[test]
    fn a_u64_beyond_i64_saturates_at_the_largest_int() {
        assert_eq!(
            parse_value(&Variant::from(u64::MAX)),
            DBusValue::Int(i64::MAX)
        );
    }

    #[test]
    fn strings_floats_and_bools_keep_their_kind() {
        assert_eq!(
            parse_value(&Variant::from("text")),
            DBusValue::String("text".to_string())
        );
        assert_eq!(parse_value(&Variant::from(1.5_f64)), DBusValue::Float(1.5));
        assert_eq!(parse_value(&Variant::from(true)), DBusValue::Bool(true));
    }

    #[test]
    fn an_array_keeps_its_items_in_order() {
        assert_eq!(
            parse_value(&Variant::from(vec![2_i32, 1])),
            DBusValue::Array(vec![DBusValue::Int(2), DBusValue::Int(1)])
        );
    }

    #[test]
    fn a_dict_keeps_its_string_keyed_entries() {
        let dict = HashMap::from([("one".to_string(), 1_i32), ("two".to_string(), 2)]);
        assert_eq!(
            parse_value(&Variant::from(dict)),
            DBusValue::Dict(HashMap::from([
                ("one".to_string(), DBusValue::Int(1)),
                ("two".to_string(), DBusValue::Int(2)),
            ]))
        );
    }

    #[test]
    fn a_dict_drops_the_entries_whose_key_is_not_a_string() {
        let dict = HashMap::from([(1_i32, "one".to_string())]);
        assert_eq!(
            parse_value(&Variant::from(dict)),
            DBusValue::Dict(HashMap::new())
        );
    }

    #[test]
    fn a_nested_variant_is_unwrapped() {
        let nested = Variant::Variant(Box::new(Variant::from(9_u32)));
        assert_eq!(parse_value(&nested), DBusValue::Int(9));
    }

    #[test]
    fn an_unsupported_kind_becomes_null() {
        assert_eq!(parse_value(&Variant::from(5_u8)), DBusValue::Null);
    }
}
