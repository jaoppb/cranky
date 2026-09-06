use super::icon::SystrayIcon;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(transparent)]
pub struct SystrayTooltipTitle(String);

impl SystrayTooltipTitle {
    pub fn new(title: impl Into<String>) -> Self {
        Self(title.into())
    }
    #[cfg(test)]
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(transparent)]
pub struct SystrayTooltipDescription(String);

impl SystrayTooltipDescription {
    pub fn new(desc: impl Into<String>) -> Self {
        Self(desc.into())
    }
    #[cfg(test)]
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SystrayTooltip {
    icon: Option<SystrayIcon>,
    title: SystrayTooltipTitle,
    description: SystrayTooltipDescription,
}

impl SystrayTooltip {
    #[must_use]
    pub const fn new(
        icon: Option<SystrayIcon>,
        title: SystrayTooltipTitle,
        description: SystrayTooltipDescription,
    ) -> Self {
        Self {
            icon,
            title,
            description,
        }
    }

    #[cfg(test)]
    #[must_use]
    pub const fn icon(&self) -> Option<&SystrayIcon> {
        self.icon.as_ref()
    }

    #[cfg(test)]
    #[must_use]
    pub const fn title(&self) -> &SystrayTooltipTitle {
        &self.title
    }

    #[cfg(test)]
    #[must_use]
    pub const fn description(&self) -> &SystrayTooltipDescription {
        &self.description
    }
}
