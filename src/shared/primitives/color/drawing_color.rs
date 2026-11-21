use super::color_struct::Color;
use super::error::ColorError;
use super::parser::{parse_single_color, tokenize};
use serde::{Deserialize, Deserializer};

/// A paint for fills, borders and text: one color, or a linear gradient.
#[derive(Debug, Clone, PartialEq)]
pub enum DrawingColor {
    /// A single uniform color.
    Solid(Color),
    /// A linear gradient through two or more evenly spread colors, at an angle in degrees.
    Gradient(Vec<Color>, f32),
}

impl Default for DrawingColor {
    fn default() -> Self {
        Self::Solid(Color::new(0, 0, 0, 255))
    }
}

impl<'de> Deserialize<'de> for DrawingColor {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let text = String::deserialize(deserializer)?;
        Self::parse(&text).map_err(serde::de::Error::custom)
    }
}

impl DrawingColor {
    /// Parses a drawing color from a string.
    ///
    /// # Errors
    ///
    /// Returns `ColorError` if input is empty, no colors found, invalid format, or invalid angle.
    pub(crate) fn parse(input: &str) -> Result<Self, ColorError> {
        let input = input.trim();
        if input.is_empty() {
            return Err(ColorError::Empty);
        }

        let tokens = tokenize(input);
        if tokens.is_empty() {
            return Err(ColorError::NoColors(input.to_string()));
        }

        let mut colors = Vec::new();
        let mut angle = 0.0;

        for (index, token) in tokens.iter().enumerate() {
            if let Some(color) = parse_single_color(token) {
                colors.push(color);
            } else if index == tokens.len().saturating_sub(1) && tokens.len() > 1 {
                let angle_str = token.strip_suffix("deg").unwrap_or(token);
                let Ok(parsed_angle) = angle_str.parse::<f32>() else {
                    return Err(ColorError::InvalidAngle(token.clone()));
                };
                angle = parsed_angle;
            } else {
                return Err(ColorError::InvalidFormat(token.clone()));
            }
        }

        if colors.is_empty() {
            return Err(ColorError::NoColors(input.to_string()));
        }

        if colors.len() > 1 {
            Ok(Self::Gradient(colors, angle))
        } else if let Some(color) = colors.into_iter().next() {
            Ok(Self::Solid(color))
        } else {
            Err(ColorError::NoColors(input.to_string()))
        }
    }
}
