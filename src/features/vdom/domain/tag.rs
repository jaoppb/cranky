//! Node tags (the element names CSS selects, such as `flex` and `text`) and the string a text node
//! shows.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum NodeTag {
    Flex,
    Grid,
    Text,
    Progress,
    Rect,
    Image,
    Module,
}

impl NodeTag {
    #[must_use]
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Flex => "flex",
            Self::Grid => "grid",
            Self::Text => "text",
            Self::Progress => "progress",
            Self::Rect => "rect",
            Self::Image => "image",
            Self::Module => "module",
        }
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) const fn is_container(self) -> bool {
        matches!(self, Self::Flex | Self::Grid)
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) const fn is_leaf(self) -> bool {
        !self.is_container()
    }
}

impl std::fmt::Display for NodeTag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The string a text node displays, kept exactly as the script gave it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct TextContent {
    text: String,
}

impl TextContent {
    /// Wraps a script's string as text content, unchanged.
    #[must_use]
    pub const fn new(text: String) -> Self {
        Self { text }
    }

    #[must_use]
    pub(crate) fn as_str(&self) -> &str {
        &self.text
    }
}

impl std::str::FromStr for TextContent {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::new(s.to_string()))
    }
}

impl From<&str> for TextContent {
    fn from(text: &str) -> Self {
        Self::new(text.to_string())
    }
}

impl From<String> for TextContent {
    fn from(text: String) -> Self {
        Self::new(text)
    }
}

impl std::fmt::Display for TextContent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_content() {
        let text = TextContent::new("Hello World".to_string());
        assert_eq!(text.as_str(), "Hello World");
        assert_eq!(text.to_string(), "Hello World");
    }
}
