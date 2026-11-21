//! The tooltip a tray item asks to show: an optional icon, a title and a description.
use super::icon::SystrayIcon;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(transparent)]
pub(crate) struct SystrayTooltipTitle(String);

impl SystrayTooltipTitle {
    pub(crate) fn new(title: impl Into<String>) -> Self {
        Self(title.into())
    }
    #[cfg(test)]
    #[must_use]
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(transparent)]
pub(crate) struct SystrayTooltipDescription(String);

impl SystrayTooltipDescription {
    pub(crate) fn new(desc: impl Into<String>) -> Self {
        Self(desc.into())
    }
    #[cfg(test)]
    #[must_use]
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct SystrayTooltip {
    icon: Option<SystrayIcon>,
    title: SystrayTooltipTitle,
    description: SystrayTooltipDescription,
}

impl SystrayTooltip {
    #[must_use]
    pub(crate) const fn new(
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
    pub(crate) const fn icon(&self) -> Option<&SystrayIcon> {
        self.icon.as_ref()
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) const fn title(&self) -> &SystrayTooltipTitle {
        &self.title
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) const fn description(&self) -> &SystrayTooltipDescription {
        &self.description
    }
}
