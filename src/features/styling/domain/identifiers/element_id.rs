use super::super::errors::StylingError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct ElementId(String);

impl ElementId {
    /// Creates a new `ElementId`.
    ///
    /// # Errors
    ///
    /// Returns `StylingError::InvalidElementId` if id is empty or contains invalid characters.
    pub fn new(id: impl Into<String>) -> Result<Self, StylingError> {
        let element_id = id.into();
        if element_id.is_empty() {
            return Err(StylingError::InvalidElementId(element_id));
        }
        let Some(first) = element_id.chars().next() else {
            return Err(StylingError::InvalidElementId(element_id));
        };
        if !first.is_ascii_alphabetic() && first != '_' {
            return Err(StylingError::InvalidElementId(element_id));
        }
        if !element_id.chars().all(|character| {
            character.is_ascii_alphanumeric() || character == '_' || character == '-'
        }) {
            return Err(StylingError::InvalidElementId(element_id));
        }
        Ok(Self(element_id))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ElementId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<'de> Deserialize<'de> for ElementId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let element_id = String::deserialize(deserializer)?;
        Self::new(element_id).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_element_id_validation() {
        assert!(ElementId::new("hour-main").is_ok());
        assert!(ElementId::new("ws_1").is_ok());
        assert!(ElementId::new("").is_err());
        assert!(ElementId::new("1ws").is_err());
        assert!(ElementId::new("#ws").is_err());
    }
}
