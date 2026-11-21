use serde::Deserialize;

use crate::shared::config::domain::{self, MarginOffset};

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum MarginConfigDto {
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
    pub const fn into_domain(self) -> domain::MarginConfig {
        match self {
            Self::All(val) => domain::MarginConfig::new(
                MarginOffset::new(val),
                MarginOffset::new(val),
                MarginOffset::new(val),
                MarginOffset::new(val),
            ),
            Self::Fields {
                top,
                bottom,
                left,
                right,
                horizontal,
                vertical,
            } => {
                let top_offset = match top {
                    Some(value) => value,
                    None => match vertical {
                        Some(value) => value,
                        None => 0,
                    },
                };
                let bottom_offset = match bottom {
                    Some(value) => value,
                    None => match vertical {
                        Some(value) => value,
                        None => 0,
                    },
                };
                let left_offset = match left {
                    Some(value) => value,
                    None => match horizontal {
                        Some(value) => value,
                        None => 0,
                    },
                };
                let right_offset = match right {
                    Some(value) => value,
                    None => match horizontal {
                        Some(value) => value,
                        None => 0,
                    },
                };
                domain::MarginConfig::new(
                    MarginOffset::new(top_offset),
                    MarginOffset::new(bottom_offset),
                    MarginOffset::new(left_offset),
                    MarginOffset::new(right_offset),
                )
            }
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum PartialMarginConfigDto {
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
    pub const fn into_domain(self) -> domain::PartialMarginConfig {
        match self {
            Self::All(val) => domain::PartialMarginConfig::new(
                Some(MarginOffset::new(val)),
                Some(MarginOffset::new(val)),
                Some(MarginOffset::new(val)),
                Some(MarginOffset::new(val)),
            ),
            Self::Fields {
                top,
                bottom,
                left,
                right,
                horizontal,
                vertical,
            } => {
                let top_offset = match top {
                    Some(value) => Some(MarginOffset::new(value)),
                    None => match vertical {
                        Some(value) => Some(MarginOffset::new(value)),
                        None => None,
                    },
                };
                let bottom_offset = match bottom {
                    Some(value) => Some(MarginOffset::new(value)),
                    None => match vertical {
                        Some(value) => Some(MarginOffset::new(value)),
                        None => None,
                    },
                };
                let left_offset = match left {
                    Some(value) => Some(MarginOffset::new(value)),
                    None => match horizontal {
                        Some(value) => Some(MarginOffset::new(value)),
                        None => None,
                    },
                };
                let right_offset = match right {
                    Some(value) => Some(MarginOffset::new(value)),
                    None => match horizontal {
                        Some(value) => Some(MarginOffset::new(value)),
                        None => None,
                    },
                };
                domain::PartialMarginConfig::new(
                    top_offset,
                    bottom_offset,
                    left_offset,
                    right_offset,
                )
            }
        }
    }
}
