use serde::{Deserialize, Deserializer};
use thiserror::Error;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum ColorError {
    #[error("Empty color string")]
    Empty,
    #[error("No colors found in input '{0}'")]
    NoColors(String),
    #[error("Invalid color format: '{0}'")]
    InvalidFormat(String),
    #[error("Invalid angle value: '{0}'")]
    InvalidAngle(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Color {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

impl Color {
    #[must_use]
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    #[must_use]
    pub const fn r(&self) -> u8 {
        self.r
    }
    #[must_use]
    pub const fn g(&self) -> u8 {
        self.g
    }
    #[must_use]
    pub const fn b(&self) -> u8 {
        self.b
    }
    #[must_use]
    pub const fn a(&self) -> u8 {
        self.a
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum DrawingColor {
    Solid(Color),
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
        let s = String::deserialize(deserializer)?;
        Self::parse(&s).map_err(serde::de::Error::custom)
    }
}

impl DrawingColor {
    /// Parses a drawing color from a string.
    ///
    /// # Errors
    ///
    /// Returns `ColorError` if input is empty, no colors found, invalid format, or invalid angle.
    pub fn parse(input: &str) -> Result<Self, ColorError> {
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

        for (i, token) in tokens.iter().enumerate() {
            if let Some(c) = parse_single_color(token) {
                colors.push(c);
            } else if i == tokens.len().saturating_sub(1) && tokens.len() > 1 {
                let angle_str = token.strip_suffix("deg").unwrap_or(token);
                let Ok(a) = angle_str.parse::<f32>() else {
                    return Err(ColorError::InvalidAngle(token.clone()));
                };
                angle = a;
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

fn tokenize(input: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current_token = String::new();
    let mut in_parens: u32 = 0;

    for c in input.chars() {
        if c == '(' {
            in_parens = in_parens.saturating_add(1);
            current_token.push(c);
        } else if c == ')' {
            in_parens = in_parens.saturating_sub(1);
            current_token.push(c);
        } else if c.is_whitespace() && in_parens == 0 {
            if !current_token.is_empty() {
                tokens.push(current_token.clone());
                current_token.clear();
            }
        } else {
            current_token.push(c);
        }
    }
    if !current_token.is_empty() {
        tokens.push(current_token);
    }
    tokens
}

fn parse_single_color(s: &str) -> Option<Color> {
    parse_css_rgba(s)
        .or_else(|| parse_css_rgb(s))
        .or_else(|| parse_rgba_hex(s))
        .or_else(|| parse_rgb_hex(s))
        .or_else(|| parse_hex(s))
}

#[allow(clippy::many_single_char_names)]
fn parse_rgba_hex(s: &str) -> Option<Color> {
    let content = s.strip_prefix("rgba(")?.strip_suffix(')')?;
    if content.len() == 8 {
        let r = u8::from_str_radix(content.get(0..2)?, 16).ok()?;
        let g = u8::from_str_radix(content.get(2..4)?, 16).ok()?;
        let b = u8::from_str_radix(content.get(4..6)?, 16).ok()?;
        let a = u8::from_str_radix(content.get(6..8)?, 16).ok()?;
        return Some(Color::new(r, g, b, a));
    }
    None
}

fn parse_rgb_hex(s: &str) -> Option<Color> {
    let content = s.strip_prefix("rgb(")?.strip_suffix(')')?;
    if content.len() == 6 {
        let r = u8::from_str_radix(content.get(0..2)?, 16).ok()?;
        let g = u8::from_str_radix(content.get(2..4)?, 16).ok()?;
        let b = u8::from_str_radix(content.get(4..6)?, 16).ok()?;
        return Some(Color::new(r, g, b, 255));
    }
    None
}

#[allow(clippy::many_single_char_names)]
fn parse_hex(s: &str) -> Option<Color> {
    let hex = s.strip_prefix('#')?;
    if hex.len() == 6 {
        let r = u8::from_str_radix(hex.get(0..2)?, 16).ok()?;
        let g = u8::from_str_radix(hex.get(2..4)?, 16).ok()?;
        let b = u8::from_str_radix(hex.get(4..6)?, 16).ok()?;
        Some(Color::new(r, g, b, 255))
    } else if hex.len() == 8 {
        let r = u8::from_str_radix(hex.get(0..2)?, 16).ok()?;
        let g = u8::from_str_radix(hex.get(2..4)?, 16).ok()?;
        let b = u8::from_str_radix(hex.get(4..6)?, 16).ok()?;
        let a = u8::from_str_radix(hex.get(6..8)?, 16).ok()?;
        Some(Color::new(r, g, b, a))
    } else {
        None
    }
}

fn parse_css_rgb(s: &str) -> Option<Color> {
    let content = s.strip_prefix("rgb(")?.strip_suffix(')')?;
    let mut parts = content.split(',').map(str::trim);
    let r_str = parts.next()?;
    let g_str = parts.next()?;
    let b_str = parts.next()?;
    if parts.next().is_none() {
        let r = r_str.parse::<u8>().ok()?;
        let g = g_str.parse::<u8>().ok()?;
        let b = b_str.parse::<u8>().ok()?;
        return Some(Color::new(r, g, b, 255));
    }
    None
}

#[allow(
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::many_single_char_names
)]
fn parse_css_rgba(s: &str) -> Option<Color> {
    let content = s.strip_prefix("rgba(")?.strip_suffix(')')?;
    let mut parts = content.split(',').map(str::trim);
    let r_str = parts.next()?;
    let g_str = parts.next()?;
    let b_str = parts.next()?;
    let a_str = parts.next()?;
    if parts.next().is_none() {
        let r = r_str.parse::<u8>().ok()?;
        let g = g_str.parse::<u8>().ok()?;
        let b = b_str.parse::<u8>().ok()?;
        let a_f = a_str.parse::<f32>().ok()?;
        let a = (a_f * 255.0).round() as u8;
        return Some(Color::new(r, g, b, a));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_new() {
        let c = Color::new(1, 2, 3, 4);
        assert_eq!(c.r(), 1);
        assert_eq!(c.g(), 2);
        assert_eq!(c.b(), 3);
        assert_eq!(c.a(), 4);
    }

    #[test]
    fn test_parse_hex() {
        let c = DrawingColor::parse("#ff0000").unwrap();
        assert_eq!(c, DrawingColor::Solid(Color::new(255, 0, 0, 255)));

        let c = DrawingColor::parse("#00ff00ff").unwrap();
        assert_eq!(c, DrawingColor::Solid(Color::new(0, 255, 0, 255)));
    }

    #[test]
    fn test_parse_rgb_rgba() {
        let c = DrawingColor::parse("rgb(ffffff)").unwrap();
        assert_eq!(c, DrawingColor::Solid(Color::new(255, 255, 255, 255)));

        let c = DrawingColor::parse("rgba(00000080)").unwrap();
        assert_eq!(c, DrawingColor::Solid(Color::new(0, 0, 0, 128)));
    }

    #[test]
    fn test_parse_css_rgb_rgba() {
        let c = DrawingColor::parse("rgb(255, 128, 0)").unwrap();
        assert_eq!(c, DrawingColor::Solid(Color::new(255, 128, 0, 255)));

        let c = DrawingColor::parse("rgba(255, 128, 0, 0.5)").unwrap();
        assert_eq!(c, DrawingColor::Solid(Color::new(255, 128, 0, 128)));
    }

    #[test]
    fn test_parse_gradient() {
        let c = DrawingColor::parse("#ff0000 #00ff00 90deg").unwrap();
        if let DrawingColor::Gradient(colors, angle) = c {
            assert_eq!(colors.len(), 2);
            assert_eq!(colors[0], Color::new(255, 0, 0, 255));
            assert_eq!(colors[1], Color::new(0, 255, 0, 255));
            assert!((angle - 90.0).abs() < f32::EPSILON);
        } else {
            panic!("Expected gradient");
        }
    }

    #[test]
    fn test_parse_errors() {
        assert_eq!(DrawingColor::parse("  "), Err(ColorError::Empty));
        assert!(DrawingColor::parse("not-a-color").is_err());
        assert!(matches!(
            DrawingColor::parse("#ff0000 #00ff00 invalid"),
            Err(ColorError::InvalidAngle(_))
        ));
    }

    #[test]
    fn test_tokenize() {
        let tokens = tokenize("rgb(255, 255, 255) #000");
        // tokenize doesn't split inside parens
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0], "rgb(255, 255, 255)");
    }
}
