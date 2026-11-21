use super::super::errors::StylingError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub(crate) enum CssLength {
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
    pub(crate) fn px(pixels: f32) -> Result<Self, StylingError> {
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
    pub(crate) fn percent(percentage: f32) -> Result<Self, StylingError> {
        if percentage.is_nan() || percentage < 0.0 {
            return Err(StylingError::InvalidLength(format!(
                "Percentage cannot be negative or NaN, got {percentage}"
            )));
        }
        Ok(Self::Percent(percentage))
    }
}
