use crate::shared::primitives::geometry::BarHeight;

use super::margin::PartialMarginConfig;
use super::types::VerticalAlignment;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PartialRootConfig {
    height: Option<BarHeight>,
    vertical_alignment: Option<VerticalAlignment>,
    margin: Option<PartialMarginConfig>,
}

pub struct CreatePartialRootConfigCommand {
    height: Option<BarHeight>,
    vertical_alignment: Option<VerticalAlignment>,
    margin: Option<PartialMarginConfig>,
}

impl CreatePartialRootConfigCommand {
    #[must_use]
    pub const fn new(
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

    #[must_use]
    pub const fn height(&self) -> Option<BarHeight> {
        self.height
    }
    #[must_use]
    pub const fn vertical_alignment(&self) -> Option<VerticalAlignment> {
        self.vertical_alignment
    }
    #[must_use]
    pub const fn margin(&self) -> Option<&PartialMarginConfig> {
        self.margin.as_ref()
    }
}

impl PartialRootConfig {
    #[allow(clippy::needless_pass_by_value)]
    #[must_use]
    pub fn new(cmd: CreatePartialRootConfigCommand) -> Self {
        Self {
            height: cmd.height(),
            vertical_alignment: cmd.vertical_alignment(),
            margin: cmd.margin().cloned(),
        }
    }

    #[must_use]
    pub const fn height(&self) -> Option<BarHeight> {
        self.height
    }
    #[must_use]
    pub const fn vertical_alignment(&self) -> Option<VerticalAlignment> {
        self.vertical_alignment
    }
    #[must_use]
    pub const fn margin(&self) -> Option<&PartialMarginConfig> {
        self.margin.as_ref()
    }
}
