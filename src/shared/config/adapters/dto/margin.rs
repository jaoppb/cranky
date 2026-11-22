use serde::Deserialize;

use crate::shared::config::domain::{self, AxisMargins, MarginOffset};

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub(super) enum MarginConfigDto {
    All(i32),
    Fields {
        top: Option<i32>,
        bottom: Option<i32>,
        left: Option<i32>,
        right: Option<i32>,
        horizontal: Option<i32>,
        vertical: Option<i32>,
    },
}

impl Default for MarginConfigDto {
    fn default() -> Self {
        Self::All(0)
    }
}

impl MarginConfigDto {
    #[must_use]
    pub(crate) const fn into_domain(self) -> domain::MarginConfig {
        match self {
            Self::All(val) => domain::MarginConfig::new(
                AxisMargins::both(MarginOffset::new(val)),
                AxisMargins::both(MarginOffset::new(val)),
            ),
            Self::Fields {
                top,
                bottom,
                left,
                right,
                horizontal,
                vertical,
            } => domain::MarginConfig::new(
                AxisMargins::new(side_offset(top, vertical), side_offset(bottom, vertical)),
                AxisMargins::new(
                    side_offset(left, horizontal),
                    side_offset(right, horizontal),
                ),
            ),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub(super) enum PartialMarginConfigDto {
    All(i32),
    Fields {
        top: Option<i32>,
        bottom: Option<i32>,
        left: Option<i32>,
        right: Option<i32>,
        horizontal: Option<i32>,
        vertical: Option<i32>,
    },
}

impl Default for PartialMarginConfigDto {
    fn default() -> Self {
        Self::Fields {
            top: None,
            bottom: None,
            left: None,
            right: None,
            horizontal: None,
            vertical: None,
        }
    }
}

impl PartialMarginConfigDto {
    #[must_use]
    pub(crate) const fn into_domain(self) -> domain::PartialMarginConfig {
        match self {
            Self::All(val) => domain::PartialMarginConfig::new(
                AxisMargins::both(Some(MarginOffset::new(val))),
                AxisMargins::both(Some(MarginOffset::new(val))),
            ),
            Self::Fields {
                top,
                bottom,
                left,
                right,
                horizontal,
                vertical,
            } => domain::PartialMarginConfig::new(
                AxisMargins::new(
                    partial_side_offset(top, vertical),
                    partial_side_offset(bottom, vertical),
                ),
                AxisMargins::new(
                    partial_side_offset(left, horizontal),
                    partial_side_offset(right, horizontal),
                ),
            ),
        }
    }
}

/// A side's offset when only some sides are set: its own value, else its axis's, else unset.
const fn partial_side_offset(side: Option<i32>, axis: Option<i32>) -> Option<MarginOffset> {
    match (side, axis) {
        (Some(value), _) | (None, Some(value)) => Some(MarginOffset::new(value)),
        (None, None) => None,
    }
}

/// A side's offset: its own value, else its axis's, else zero.
const fn side_offset(side: Option<i32>, axis: Option<i32>) -> MarginOffset {
    match partial_side_offset(side, axis) {
        Some(offset) => offset,
        None => MarginOffset::new(0),
    }
}
