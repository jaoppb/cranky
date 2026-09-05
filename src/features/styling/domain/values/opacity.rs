use super::super::errors::StylingError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Opacity(f32);

impl Opacity {
    /// Creates a new `Opacity` between 0.0 and 1.0.
    ///
    /// # Errors
    ///
    /// Returns `StylingError::InvalidOpacity` if `value` is NaN or not in 0.0..=1.0.
    pub fn new(value: f32) -> Result<Self, StylingError> {
        if value.is_nan() || !(0.0..=1.0).contains(&value) {
            return Err(StylingError::InvalidOpacity(format!("{value}")));
        }
        Ok(Self(value))
    }

    #[must_use]
    pub const fn value(&self) -> f32 {
        self.0
    }
}

impl<'de> Deserialize<'de> for Opacity {
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
    fn test_opacity_validation() {
        assert!(Opacity::new(0.0).is_ok());
        assert!(Opacity::new(0.5).is_ok());
        assert!(Opacity::new(1.0).is_ok());
        assert!(Opacity::new(-0.1).is_err());
        assert!(Opacity::new(1.1).is_err());
        assert!(Opacity::new(f32::NAN).is_err());
    }
}

