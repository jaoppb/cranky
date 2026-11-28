//! CSS class names as scripts set them on elements, singly and as a list.
use super::super::errors::StylingError;
use serde::{Deserialize, Serialize};

/// One class an element carries, matched by `.name` selectors; always a valid CSS identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct ClassName(String);

impl ClassName {
    /// Creates a new `ClassName`.
    ///
    /// # Errors
    ///
    /// Returns `StylingError::InvalidClassName` if name is empty or contains invalid characters.
    pub fn new(name: impl Into<String>) -> Result<Self, StylingError> {
        let class_name = name.into();
        if class_name.is_empty() {
            return Err(StylingError::InvalidClassName(class_name));
        }
        let Some(first) = class_name.chars().next() else {
            return Err(StylingError::InvalidClassName(class_name));
        };
        if !first.is_ascii_alphabetic() && first != '_' && first != '-' {
            return Err(StylingError::InvalidClassName(class_name));
        }
        if !class_name.chars().all(|character| {
            character.is_ascii_alphanumeric() || character == '_' || character == '-'
        }) {
            return Err(StylingError::InvalidClassName(class_name));
        }
        Ok(Self(class_name))
    }

    #[must_use]
    pub(crate) fn as_str(&self) -> &str {
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
        let class_name = String::deserialize(deserializer)?;
        Self::new(class_name).map_err(serde::de::Error::custom)
    }
}

/// The classes of one element, in the order the script listed them.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize)]
pub struct ClassNameList(Vec<ClassName>);

impl ClassNameList {
    /// Parses a space-separated string of class names.
    ///
    /// # Errors
    ///
    /// Returns `StylingError::InvalidClassName` if any class name is invalid.
    pub(crate) fn parse(classes: &str) -> Result<Self, StylingError> {
        let mut list = Vec::new();
        for item in classes.split_whitespace() {
            list.push(ClassName::new(item)?);
        }
        Ok(Self(list))
    }

    /// Wraps classes that are already validated, without parsing a string.
    #[must_use]
    pub const fn new(list: Vec<ClassName>) -> Self {
        Self(list)
    }

    #[must_use]
    pub(crate) fn as_slice(&self) -> &[ClassName] {
        &self.0
    }

    pub(crate) fn iter(&self) -> std::slice::Iter<'_, ClassName> {
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
            ClassRepr::Str(text) => Self::parse(&text).map_err(serde::de::Error::custom),
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

    #[test]
    fn test_class_name_list_deserializes_from_a_string_or_a_list() {
        let expected = ClassNameList::new(vec![
            ClassName::new("item").unwrap(),
            ClassName::new("active").unwrap(),
        ]);
        let from_list: ClassNameList = serde_json::from_str(r#"["item", "active"]"#).unwrap();
        let from_string: ClassNameList = serde_json::from_str(r#""item active""#).unwrap();
        let empty: ClassNameList = serde_json::from_str("[]").unwrap();

        assert_eq!(from_list, expected);
        assert_eq!(from_string, expected);
        assert_eq!(empty, ClassNameList::default());
    }

    #[test]
    fn test_class_name_list_rejects_an_invalid_listed_class() {
        let error = serde_json::from_str::<ClassNameList>(r#"["item", "1bad"]"#).unwrap_err();
        assert!(
            error
                .to_string()
                .starts_with("Invalid class name '1bad': must be a valid CSS identifier"),
            "{error}"
        );
    }
}
