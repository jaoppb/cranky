use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Pure domain dynamic value representation without `serde_json` dependency
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DynamicValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<Self>),
    Map(HashMap<String, Self>),
}

impl DynamicValue {
    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s),
            _ => None,
        }
    }

    #[must_use]
    pub const fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(b) => Some(*b),
            _ => None,
        }
    }

    #[must_use]
    pub const fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Number(n) => Some(*n),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Self::Number(n) => Some(crate::utils::f64_to_i64(*n)),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_array(&self) -> Option<&[Self]> {
        match self {
            Self::Array(arr) => Some(arr),
            _ => None,
        }
    }

    #[must_use]
    pub const fn as_map(&self) -> Option<&HashMap<String, Self>> {
        match self {
            Self::Map(m) => Some(m),
            _ => None,
        }
    }
}

impl From<String> for DynamicValue {
    fn from(s: String) -> Self {
        Self::String(s)
    }
}

impl From<&str> for DynamicValue {
    fn from(s: &str) -> Self {
        Self::String(s.to_string())
    }
}

impl From<bool> for DynamicValue {
    fn from(b: bool) -> Self {
        Self::Bool(b)
    }
}

impl From<f64> for DynamicValue {
    fn from(n: f64) -> Self {
        Self::Number(n)
    }
}

impl From<i64> for DynamicValue {
    fn from(n: i64) -> Self {
        Self::Number(crate::utils::i64_to_f64(n))
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
