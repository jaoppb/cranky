use super::super::errors::StylingError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Default)]
pub struct ProgressValue(f32);

impl ProgressValue {
    /// Creates a new `ProgressValue` between 0.0 and 1.0.
    ///
    /// # Errors
    ///
    /// Returns `StylingError::InvalidProgressValue` if `value` is NaN or not in 0.0..=1.0.
    pub fn new(value: f32) -> Result<Self, StylingError> {
        if value.is_nan() || !(0.0..=1.0).contains(&value) {
            return Err(StylingError::InvalidProgressValue(format!("{value}")));
        }
        Ok(Self(value))
    }

    /// Creates a new `ProgressValue` from a percentage value (0.0 to 100.0).
    ///
    /// # Errors
    ///
    /// Returns `StylingError::InvalidProgressValue` if `pct` is NaN or not in 0.0..=100.0.
    pub fn from_percentage(pct: f32) -> Result<Self, StylingError> {
        Self::new(pct / 100.0)
    }

    #[must_use]
    pub const fn value(&self) -> f32 {
        self.0
    }
}

impl<'de> Deserialize<'de> for ProgressValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let v = f32::deserialize(deserializer)?;
        Self::new(v).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Orientation {
    #[default]
    Horizontal,
    Vertical,
}

impl Orientation {
    #[must_use]
    pub const fn is_horizontal(&self) -> bool {
        matches!(self, Self::Horizontal)
    }

    #[must_use]
    pub const fn is_vertical(&self) -> bool {
        matches!(self, Self::Vertical)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_value_validation() {
        assert!(ProgressValue::new(0.0).is_ok());
        assert!(ProgressValue::new(0.5).is_ok());
        assert!(ProgressValue::new(1.0).is_ok());
        assert!(ProgressValue::new(-0.01).is_err());
        assert!(ProgressValue::new(1.01).is_err());
        assert!(ProgressValue::new(f32::NAN).is_err());

        let from_pct = ProgressValue::from_percentage(75.0).unwrap();
        assert!((from_pct.value() - 0.75).abs() < f32::EPSILON);
    }
}

