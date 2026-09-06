use std::collections::HashMap;

use crate::shared::primitives::{DynamicValue, ModuleOptions};

pub(crate) fn default_root_name() -> String {
    "bar".to_string()
}

pub(crate) const fn default_height() -> u32 {
    30
}

pub(crate) const fn default_true() -> bool {
    true
}

pub(crate) const fn default_timebased_duration_ms() -> u64 {
    100
}

pub(crate) fn json_value_to_dynamic(v: serde_json::Value) -> DynamicValue {
    match v {
        serde_json::Value::Null => DynamicValue::Null,
        serde_json::Value::Bool(b) => DynamicValue::Bool(b),
        serde_json::Value::Number(n) => DynamicValue::Number(n.as_f64().unwrap_or(0.0)),
        serde_json::Value::String(s) => DynamicValue::String(s),
        serde_json::Value::Array(arr) => {
            DynamicValue::Array(arr.into_iter().map(json_value_to_dynamic).collect())
        }
        serde_json::Value::Object(map) => DynamicValue::Map(
            map.into_iter()
                .map(|(k, v)| (k, json_value_to_dynamic(v)))
                .collect(),
        ),
    }
}

pub(crate) fn json_map_to_options(map: HashMap<String, serde_json::Value>) -> ModuleOptions {
    ModuleOptions::new(
        map.into_iter()
            .map(|(k, v)| (k, json_value_to_dynamic(v)))
            .collect(),
    )
}
