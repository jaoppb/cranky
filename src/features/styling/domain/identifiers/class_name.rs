use super::super::errors::StylingError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct ClassName(String);

impl ClassName {
    /// Creates a new `ClassName`.
    ///
    /// # Errors
    ///
    /// Returns `StylingError::InvalidClassName` if name is empty or contains invalid characters.
    pub fn new(name: impl Into<String>) -> Result<Self, StylingError> {
        let s = name.into();
        if s.is_empty() {
            return Err(StylingError::InvalidClassName(s));
        }
        let Some(first) = s.chars().next() else {
            return Err(StylingError::InvalidClassName(s));
        };
        if !first.is_ascii_alphabetic() && first != '_' && first != '-' {
            return Err(StylingError::InvalidClassName(s));
        }
        if !s
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            return Err(StylingError::InvalidClassName(s));
        }
        Ok(Self(s))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ClassName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<'de> Deserialize<'de> for ClassName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Self::new(s).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize)]
pub struct ClassNameList(Vec<ClassName>);

impl ClassNameList {
    /// Parses a space-separated string of class names.
    ///
    /// # Errors
    ///
    /// Returns `StylingError::InvalidClassName` if any class name is invalid.
    pub fn parse(classes: &str) -> Result<Self, StylingError> {
        let mut list = Vec::new();
        for item in classes.split_whitespace() {
            list.push(ClassName::new(item)?);
        }
        Ok(Self(list))
    }

    #[must_use]
    pub const fn new(list: Vec<ClassName>) -> Self {
        Self(list)
    }

    #[must_use]
    pub fn as_slice(&self) -> &[ClassName] {
        &self.0
    }

    pub fn iter(&self) -> std::slice::Iter<'_, ClassName> {
        self.0.iter()
    }
}

impl<'a> IntoIterator for &'a ClassNameList {
    type Item = &'a ClassName;
    type IntoIter = std::slice::Iter<'a, ClassName>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'de> Deserialize<'de> for ClassNameList {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum ClassRepr {
            Str(String),
            List(Vec<String>),
        }

        match ClassRepr::deserialize(deserializer)? {
            ClassRepr::Str(s) => Self::parse(&s).map_err(serde::de::Error::custom),
            ClassRepr::List(list) => {
                let mut out = Vec::new();
                for item in list {
                    out.push(ClassName::new(item).map_err(serde::de::Error::custom)?);
                }
                Ok(Self(out))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_class_name_validation() {
        assert!(ClassName::new("workspace-btn").is_ok());
        assert!(ClassName::new("_active").is_ok());
        assert!(ClassName::new("item1").is_ok());
        assert!(ClassName::new("").is_err());
        assert!(ClassName::new("123item").is_err());
        assert!(ClassName::new("btn.active").is_err());
        assert!(ClassName::new("btn active").is_err());
    }
}
