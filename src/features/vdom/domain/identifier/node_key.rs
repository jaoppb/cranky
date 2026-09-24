use crate::features::vdom::domain::errors::VdomError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct NodeKey(String);

impl NodeKey {
    /// Creates a new `NodeKey`.
    ///
    /// # Errors
    ///
    /// Returns `VdomError::InvalidNodeKey` if the key is empty or whitespace.
    pub fn new(key: impl Into<String>) -> Result<Self, VdomError> {
        let s = key.into();
        if s.trim().is_empty() {
            return Err(VdomError::InvalidNodeKey(
                "NodeKey cannot be empty or whitespace".to_string(),
            ));
        }
        Ok(Self(s))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for NodeKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<'de> Deserialize<'de> for NodeKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Self::new(s).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_key_validation() {
        assert!(NodeKey::new("valid-key_123").is_ok());
        assert!(NodeKey::new("").is_err());
        assert!(NodeKey::new("   ").is_err());

        let key = NodeKey::new("tab-1").unwrap();
        assert_eq!(key.as_str(), "tab-1");
        assert_eq!(key.to_string(), "tab-1");
    }
}
