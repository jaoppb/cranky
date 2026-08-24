pub mod color;
pub mod geometry;
pub mod render;

use crate::shared::primitives::geometry::{Rect, Size};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

/// Pure domain dynamic value representation without serde_json dependency
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DynamicValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<DynamicValue>),
    Map(HashMap<String, DynamicValue>),
}

impl DynamicValue {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s),
            _ => None,
        }
    }
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(b) => Some(*b),
            _ => None,
        }
    }
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Number(n) => Some(*n),
            _ => None,
        }
    }
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Self::Number(n) => Some(*n as i64),
            _ => None,
        }
    }
    pub fn as_array(&self) -> Option<&[DynamicValue]> {
        match self {
            Self::Array(arr) => Some(arr),
            _ => None,
        }
    }
    pub fn as_map(&self) -> Option<&HashMap<String, DynamicValue>> {
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
        Self::Number(n as f64)
    }
}

impl From<Vec<DynamicValue>> for DynamicValue {
    fn from(arr: Vec<DynamicValue>) -> Self {
        Self::Array(arr)
    }
}

impl From<HashMap<String, DynamicValue>> for DynamicValue {
    fn from(map: HashMap<String, DynamicValue>) -> Self {
        Self::Map(map)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ModuleId(u32);

impl ModuleId {
    pub fn new(id: u32) -> Self {
        Self(id)
    }
    pub fn value(&self) -> u32 {
        self.0
    }
}
impl fmt::Display for ModuleId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MonitorId(String);

impl MonitorId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl fmt::Display for MonitorId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ModuleName(String);

impl ModuleName {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for ModuleName {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for ModuleName {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl PartialEq<str> for ModuleName {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}

impl PartialEq<&str> for ModuleName {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}

impl fmt::Display for ModuleName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier for an instance of a module within a container
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ModuleInstanceId(String);

impl ModuleInstanceId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ModuleInstanceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for ModuleInstanceId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for ModuleInstanceId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

/// Strongly-typed key identifying a module invocation (name + optional instance ID)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ModuleKey {
    name: ModuleName,
    instance_id: Option<ModuleInstanceId>,
}

impl ModuleKey {
    pub fn new(name: ModuleName, instance_id: Option<ModuleInstanceId>) -> Self {
        Self { name, instance_id }
    }

    pub fn from_name(name: impl Into<ModuleName>) -> Self {
        Self {
            name: name.into(),
            instance_id: None,
        }
    }

    pub fn name(&self) -> &ModuleName {
        &self.name
    }

    pub fn instance_id(&self) -> Option<&ModuleInstanceId> {
        self.instance_id.as_ref()
    }
}

impl fmt::Display for ModuleKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(id) = &self.instance_id {
            write!(f, "{}:{}", self.name, id)
        } else {
            write!(f, "{}", self.name)
        }
    }
}

/// Encapsulated options map passed to module instances
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ModuleOptions(HashMap<String, DynamicValue>);

impl ModuleOptions {
    pub fn new(map: HashMap<String, DynamicValue>) -> Self {
        Self(map)
    }

    pub fn as_map(&self) -> &HashMap<String, DynamicValue> {
        &self.0
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn get(&self, key: &str) -> Option<&DynamicValue> {
        self.0.get(key)
    }
}

/// Strongly-typed layout descriptor for a child module in a container
#[derive(Debug, Clone, PartialEq)]
pub struct ChildModuleLayout {
    key: ModuleKey,
    bounds: Rect,
}

impl ChildModuleLayout {
    pub fn new(key: ModuleKey, bounds: Rect) -> Self {
        Self { key, bounds }
    }

    pub fn key(&self) -> &ModuleKey {
        &self.key
    }

    pub fn bounds(&self) -> &Rect {
        &self.bounds
    }
}

/// Strongly-typed map of child module sizes per monitor
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ChildSizesMap(HashMap<ModuleKey, Size>);

impl ChildSizesMap {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn insert(&mut self, key: ModuleKey, size: Size) {
        self.0.insert(key, size);
    }

    pub fn get(&self, key: &ModuleKey) -> Option<&Size> {
        self.0.get(key)
    }

    pub fn get_by_name_or_key(
        &self,
        name: &ModuleName,
        instance_id: Option<&ModuleInstanceId>,
    ) -> Option<&Size> {
        let key = ModuleKey::new(name.clone(), instance_id.cloned());
        self.0.get(&key).or_else(|| {
            if instance_id.is_some() {
                self.0.get(&ModuleKey::new(name.clone(), None))
            } else {
                None
            }
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
pub struct FunctionName(String);

impl FunctionName {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for FunctionName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dynamic_value_and_module_options() {
        let mut map = HashMap::new();
        map.insert("string_key".to_string(), DynamicValue::from("hello"));
        map.insert("bool_key".to_string(), DynamicValue::from(true));
        map.insert("num_key".to_string(), DynamicValue::from(42.0));
        map.insert(
            "arr_key".to_string(),
            DynamicValue::from(vec![DynamicValue::from("item1")]),
        );

        let options = ModuleOptions::new(map);
        assert_eq!(options.get("string_key").unwrap().as_str(), Some("hello"));
        assert_eq!(options.get("bool_key").unwrap().as_bool(), Some(true));
        assert_eq!(options.get("num_key").unwrap().as_f64(), Some(42.0));
        assert_eq!(options.get("num_key").unwrap().as_i64(), Some(42));
        assert_eq!(options.get("arr_key").unwrap().as_array().unwrap().len(), 1);
        assert!(!options.is_empty());
    }

    #[test]
    fn test_module_key_and_child_sizes() {
        let key1 = ModuleKey::new(
            ModuleName::new("workspace"),
            Some(ModuleInstanceId::new("ws1")),
        );
        let key2 = ModuleKey::from_name("hour");

        assert_eq!(key1.to_string(), "workspace:ws1");
        assert_eq!(key2.to_string(), "hour");

        let mut sizes = ChildSizesMap::new();
        sizes.insert(key1.clone(), Size::new(100, 30));
        sizes.insert(key2.clone(), Size::new(80, 25));

        assert_eq!(sizes.get(&key1), Some(&Size::new(100, 30)));
        assert_eq!(
            sizes.get_by_name_or_key(&ModuleName::new("hour"), None),
            Some(&Size::new(80, 25))
        );
        assert_eq!(
            sizes.get_by_name_or_key(
                &ModuleName::new("workspace"),
                Some(&ModuleInstanceId::new("ws1"))
            ),
            Some(&Size::new(100, 30))
        );
    }
}
