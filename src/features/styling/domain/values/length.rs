use super::super::errors::StylingError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum CssLength {
    Px(f32),
    Percent(f32),
    Auto,
}

impl CssLength {
    /// Creates a pixel length value.
    ///
    /// # Errors
    ///
    /// Returns `StylingError::InvalidLength` if `v` is NaN or negative.
    pub fn px(v: f32) -> Result<Self, StylingError> {
        if v.is_nan() || v < 0.0 {
            return Err(StylingError::InvalidLength(format!(
                "Length cannot be negative or NaN, got {v}"
            )));
        }
        Ok(Self::Px(v))
    }

    /// Creates a percentage length value.
    ///
    /// # Errors
    ///
    /// Returns `StylingError::InvalidLength` if `v` is NaN or negative.
    pub fn percent(v: f32) -> Result<Self, StylingError> {
        if v.is_nan() || v < 0.0 {
            return Err(StylingError::InvalidLength(format!(
                "Percentage cannot be negative or NaN, got {v}"
            )));
        }
        Ok(Self::Percent(v))
    }

    #[must_use]
    pub const fn value(&self) -> Option<f32> {
        match self {
            Self::Px(v) | Self::Percent(v) => Some(*v),
            Self::Auto => None,
        }
    }

    #[must_use]
    pub const fn is_auto(&self) -> bool {
        matches!(self, Self::Auto)
    }
}
