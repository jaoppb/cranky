use super::super::errors::StylingError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct FlexGrow(f32);

impl FlexGrow {
    /// Creates a new `FlexGrow` value >= 0.0.
    ///
    /// # Errors
    ///
    /// Returns `StylingError::InvalidFlexValue` if `value` is NaN or negative.
    pub fn new(value: f32) -> Result<Self, StylingError> {
        if value.is_nan() || value < 0.0 {
            return Err(StylingError::InvalidFlexValue(format!(
                "flex-grow cannot be negative or NaN, got {value}"
            )));
        }
        Ok(Self(value))
    }

    #[must_use]
    pub const fn value(&self) -> f32 {
        self.0
    }
}

impl<'de> Deserialize<'de> for FlexGrow {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let v = f32::deserialize(deserializer)?;
        Self::new(v).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct FlexShrink(f32);

impl FlexShrink {
    /// Creates a new `FlexShrink` value >= 0.0.
    ///
    /// # Errors
    ///
    /// Returns `StylingError::InvalidFlexValue` if `value` is NaN or negative.
    pub fn new(value: f32) -> Result<Self, StylingError> {
        if value.is_nan() || value < 0.0 {
            return Err(StylingError::InvalidFlexValue(format!(
                "flex-shrink cannot be negative or NaN, got {value}"
            )));
        }
        Ok(Self(value))
    }

    #[must_use]
    pub const fn value(&self) -> f32 {
        self.0
    }
}

impl<'de> Deserialize<'de> for FlexShrink {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let v = f32::deserialize(deserializer)?;
        Self::new(v).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flex_vos_validation() {
        assert!(FlexGrow::new(0.0).is_ok());
        assert!(FlexGrow::new(1.5).is_ok());
        assert!(FlexGrow::new(-1.0).is_err());
        assert!(FlexGrow::new(f32::NAN).is_err());

        assert!(FlexShrink::new(0.0).is_ok());
        assert!(FlexShrink::new(2.0).is_ok());
        assert!(FlexShrink::new(-0.5).is_err());
        assert!(FlexShrink::new(f32::NAN).is_err());
    }
}

