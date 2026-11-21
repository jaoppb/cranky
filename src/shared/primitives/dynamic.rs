use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Pure domain dynamic value representation without `serde_json` dependency
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub(crate) enum DynamicValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<Self>),
    Map(HashMap<String, Self>),
}

impl DynamicValue {
    #[must_use]
    pub(crate) fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(text) => Some(text),
            _ => None,
        }
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) const fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(flag) => Some(*flag),
            _ => None,
        }
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) const fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Number(number) => Some(*number),
            _ => None,
        }
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) fn as_i64(&self) -> Option<i64> {
        match self {
            Self::Number(number) => Some(crate::utils::f64_to_i64(*number)),
            _ => None,
        }
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) fn as_array(&self) -> Option<&[Self]> {
        match self {
            Self::Array(arr) => Some(arr),
            _ => None,
        }
    }
}

impl From<String> for DynamicValue {
    fn from(text: String) -> Self {
        Self::String(text)
    }
}

impl From<&str> for DynamicValue {
    fn from(text: &str) -> Self {
        Self::String(text.to_string())
    }
}

impl From<bool> for DynamicValue {
    fn from(flag: bool) -> Self {
        Self::Bool(flag)
    }
}

impl From<f64> for DynamicValue {
    fn from(number: f64) -> Self {
        Self::Number(number)
    }
}

impl From<i64> for DynamicValue {
    fn from(number: i64) -> Self {
        Self::Number(crate::utils::i64_to_f64(number))
    }
}

impl From<Vec<Self>> for DynamicValue {
    fn from(arr: Vec<Self>) -> Self {
        Self::Array(arr)
    }
}

impl From<HashMap<String, Self>> for DynamicValue {
    fn from(map: HashMap<String, Self>) -> Self {
        Self::Map(map)
    }
}
