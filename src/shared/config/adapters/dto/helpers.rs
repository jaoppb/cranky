use std::collections::HashMap;

use crate::shared::primitives::{DynamicValue, ModuleOptions};

pub(super) fn default_root_name() -> String {
    "bar".to_string()
}

pub(super) const fn default_height() -> u32 {
    30
}

pub(super) const fn default_true() -> bool {
    true
}

pub(super) const fn default_timebased_duration_ms() -> u64 {
    100
}

pub(super) fn json_value_to_dynamic(json_value: serde_json::Value) -> DynamicValue {
    match json_value {
        serde_json::Value::Null => DynamicValue::Null,
        serde_json::Value::Bool(boolean) => DynamicValue::Bool(boolean),
        serde_json::Value::Number(number) => DynamicValue::Number(number.as_f64().unwrap_or(0.0)),
        serde_json::Value::String(text) => DynamicValue::String(text),
        serde_json::Value::Array(arr) => {
            DynamicValue::Array(arr.into_iter().map(json_value_to_dynamic).collect())
        }
        serde_json::Value::Object(map) => DynamicValue::Map(
            map.into_iter()
                .map(|(key, value)| (key, json_value_to_dynamic(value)))
                .collect(),
        ),
    }
}

pub(super) fn json_map_to_options(map: HashMap<String, serde_json::Value>) -> ModuleOptions {
    ModuleOptions::new(
        map.into_iter()
            .map(|(key, value)| (key, json_value_to_dynamic(value)))
            .collect(),
    )
}
