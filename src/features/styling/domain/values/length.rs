use super::super::errors::StylingError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum CssLength {
    Px(f32),
    Percent(f32),
    Auto,
    /// A `calc()` expression mixing a percentage with an absolute offset —
    /// `calc(<percent>% + <px>px)` after every other unit (`em`/`rem`/plain
    /// numbers) has already been folded into `px` at style-cascade time.
    /// Unlike `Px`/`Percent`, this can't collapse to a single number without
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
            Self::Auto | Self::Calc { .. } => None,
        }
    }

    #[must_use]
    pub const fn is_auto(&self) -> bool {
        matches!(self, Self::Auto)
    }
}
