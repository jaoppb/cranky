use super::super::errors::StylingError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum CssLength {
    #[serde(rename = "Px")]
    Pixels(f32),
    Percent(f32),
    Auto,
    /// A `calc()` expression mixing a percentage with an absolute offset —
    /// `calc(<percent>% + <px>px)` after every other unit (`em`/`rem`/plain
    /// numbers) has already been folded into `px` at style-cascade time.
    /// Unlike `Pixels`/`Percent`, this can't collapse to a single number without
    /// knowing the containing block's size, so the layout engine resolves it
    /// in a second layout pass rather than at style time.
    Calc {
        percent: f32,
        px: f32,
    },
}

impl CssLength {
    /// Creates a pixel length value.
    ///
    /// # Errors
    ///
    /// Returns `StylingError::InvalidLength` if `pixels` is NaN or negative.
    pub fn px(pixels: f32) -> Result<Self, StylingError> {
        if pixels.is_nan() || pixels < 0.0 {
            return Err(StylingError::InvalidLength(format!(
                "Length cannot be negative or NaN, got {pixels}"
            )));
        }
        Ok(Self::Pixels(pixels))
    }

    /// Creates a percentage length value.
    ///
    /// # Errors
    ///
    /// Returns `StylingError::InvalidLength` if `percentage` is NaN or negative.
    pub fn percent(percentage: f32) -> Result<Self, StylingError> {
        if percentage.is_nan() || percentage < 0.0 {
            return Err(StylingError::InvalidLength(format!(
                "Percentage cannot be negative or NaN, got {percentage}"
            )));
        }
        Ok(Self::Percent(percentage))
    }

    #[must_use]
    pub const fn value(&self) -> Option<f32> {
        match self {
            Self::Pixels(value) | Self::Percent(value) => Some(*value),
            Self::Auto | Self::Calc { .. } => None,
        }
    }

    #[must_use]
    pub const fn is_auto(&self) -> bool {
        matches!(self, Self::Auto)
    }
}
