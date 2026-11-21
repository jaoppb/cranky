//! Root settings that `[root.unfocused]` overrides on monitors Hyprland does not report as focused.
use crate::shared::primitives::geometry::BarHeight;

use super::margin::PartialMarginConfig;
use super::types::VerticalAlignment;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct PartialRootConfig {
    height: Option<BarHeight>,
    vertical_alignment: Option<VerticalAlignment>,
    margin: Option<PartialMarginConfig>,
}

pub(crate) struct CreatePartialRootConfigCommand {
    height: Option<BarHeight>,
    vertical_alignment: Option<VerticalAlignment>,
    margin: Option<PartialMarginConfig>,
}

impl CreatePartialRootConfigCommand {
    #[must_use]
    pub(crate) const fn new(
        height: Option<BarHeight>,
        vertical_alignment: Option<VerticalAlignment>,
        margin: Option<PartialMarginConfig>,
    ) -> Self {
        Self {
            height,
            vertical_alignment,
            margin,
        }
    }
}

impl PartialRootConfig {
    #[must_use]
    pub(crate) const fn new(cmd: CreatePartialRootConfigCommand) -> Self {
        Self {
            height: cmd.height,
            vertical_alignment: cmd.vertical_alignment,
            margin: cmd.margin,
        }
    }

    #[must_use]
    pub(crate) const fn height(&self) -> Option<BarHeight> {
        self.height
    }
    #[must_use]
    pub(crate) const fn vertical_alignment(&self) -> Option<VerticalAlignment> {
        self.vertical_alignment
    }
    #[must_use]
    pub(crate) const fn margin(&self) -> Option<&PartialMarginConfig> {
        self.margin.as_ref()
    }
}
