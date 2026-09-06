use crate::shared::primitives::BinaryData;
use crate::shared::primitives::geometry::Size;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct IconName(String);

impl IconName {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct IconThemePath(String);

impl IconThemePath {
    pub fn new(path: impl Into<String>) -> Self {
        Self(path.into())
    }
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IconCacheKey {
    name: IconName,
    theme_path: Option<IconThemePath>,
}

impl IconCacheKey {
    #[must_use]
    pub const fn new(name: IconName, theme_path: Option<IconThemePath>) -> Self {
        Self { name, theme_path }
    }
    #[must_use]
    pub const fn name(&self) -> &IconName {
        &self.name
    }
    #[must_use]
    pub const fn theme_path(&self) -> Option<&IconThemePath> {
        self.theme_path.as_ref()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IconImage {
    data: BinaryData,
    size: Size,
}

impl IconImage {
    pub fn new(data: impl Into<BinaryData>, size: Size) -> Self {
        Self {
            data: data.into(),
            size,
        }
    }
    #[cfg(test)]
    #[must_use]
    pub fn data(&self) -> &[u8] {
        &self.data
    }
    #[cfg(test)]
    #[must_use]
    pub const fn size(&self) -> &Size {
        &self.size
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum SystrayIcon {
    Both { name: IconName, image: IconImage },
    NameOnly { name: IconName },
    ImageOnly { image: IconImage },
}

impl SystrayIcon {
    #[must_use]
    pub fn new(name: Option<IconName>, image: Option<IconImage>) -> Option<Self> {
        match (name, image) {
            (Some(name), Some(image)) => Some(Self::Both { name, image }),
            (Some(name), None) => Some(Self::NameOnly { name }),
            (None, Some(image)) => Some(Self::ImageOnly { image }),
            (None, None) => None,
        }
    }

    #[cfg(test)]
    #[must_use]
    pub const fn name(&self) -> Option<&IconName> {
        match self {
            Self::Both { name, .. } | Self::NameOnly { name } => Some(name),
            Self::ImageOnly { .. } => None,
        }
    }

    #[cfg(test)]
    #[must_use]
    pub const fn image(&self) -> Option<&IconImage> {
        match self {
            Self::Both { image, .. } | Self::ImageOnly { image } => Some(image),
            Self::NameOnly { .. } => None,
        }
    }
}
