use super::super::errors::StylingError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StyleSheetName(String);

impl StyleSheetName {
    /// Creates a new `StyleSheetName`.
    ///
    /// # Errors
    ///
    /// Returns `StylingError::InvalidStyleSheetName` if name is empty, contains path separators, `.css` extension, or invalid characters.
    pub fn new(name: impl Into<String>) -> Result<Self, StylingError> {
        let s = name.into();
        if s.is_empty() {
            return Err(StylingError::InvalidStyleSheetName(s));
        }
        if s.contains('/')
            || s.contains('\\')
            || s.contains("..")
            || std::path::Path::new(&s)
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("css"))
        {
            return Err(StylingError::InvalidStyleSheetName(s));
        }
        if !s
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            return Err(StylingError::InvalidStyleSheetName(s));
        }
        Ok(Self(s))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for StyleSheetName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stylesheet_name_validation() {
        assert!(StyleSheetName::new("base").is_ok());
        assert!(StyleSheetName::new("hour_style-1").is_ok());
        assert_eq!(
            StyleSheetName::new("").unwrap_err(),
            StylingError::InvalidStyleSheetName(String::new())
        );
        assert!(StyleSheetName::new("base.css").is_err());
        assert!(StyleSheetName::new("../base").is_err());
        assert!(StyleSheetName::new("styles/base").is_err());
        assert!(StyleSheetName::new("base name").is_err());
    }
}
