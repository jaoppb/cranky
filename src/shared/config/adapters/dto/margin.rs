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
                let t = match top {
                    Some(v) => v,
                    None => match vertical {
                        Some(v) => v,
                        None => 0,
                    },
                };
                let b = match bottom {
                    Some(v) => v,
                    None => match vertical {
                        Some(v) => v,
                        None => 0,
                    },
                };
                let l = match left {
                    Some(v) => v,
                    None => match horizontal {
                        Some(v) => v,
                        None => 0,
                    },
                };
                let r = match right {
                    Some(v) => v,
                    None => match horizontal {
                        Some(v) => v,
                        None => 0,
                    },
                };
                domain::MarginConfig::new(
                    MarginOffset::new(t),
                    MarginOffset::new(b),
                    MarginOffset::new(l),
                    MarginOffset::new(r),
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
                let t = match top {
                    Some(v) => Some(MarginOffset::new(v)),
                    None => match vertical {
                        Some(v) => Some(MarginOffset::new(v)),
                        None => None,
                    },
                };
                let b = match bottom {
                    Some(v) => Some(MarginOffset::new(v)),
                    None => match vertical {
                        Some(v) => Some(MarginOffset::new(v)),
                        None => None,
                    },
                };
                let l = match left {
                    Some(v) => Some(MarginOffset::new(v)),
                    None => match horizontal {
                        Some(v) => Some(MarginOffset::new(v)),
                        None => None,
                    },
                };
                let r = match right {
                    Some(v) => Some(MarginOffset::new(v)),
                    None => match horizontal {
                        Some(v) => Some(MarginOffset::new(v)),
                        None => None,
                    },
                };
                domain::PartialMarginConfig::new(t, b, l, r)
            }
        }
    }
}
