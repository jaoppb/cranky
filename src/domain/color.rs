use serde::{Deserialize, Deserializer};
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

impl Color {
    pub fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub fn r(&self) -> u8 { self.r }
    pub fn g(&self) -> u8 { self.g }
    pub fn b(&self) -> u8 { self.b }
    pub fn a(&self) -> u8 { self.a }
}

#[derive(Debug, Clone, PartialEq)]
pub enum DrawingColor {
    Solid(Color),
    Gradient(Vec<Color>, f32),
}

impl Default for DrawingColor {
    fn default() -> Self {
        DrawingColor::Solid(Color::new(0, 0, 0, 255))
    }
}

impl<'de> Deserialize<'de> for DrawingColor {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        DrawingColor::parse(&s).map_err(serde::de::Error::custom)
    }
}

impl DrawingColor {
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
            } else if i == tokens.len() - 1 && tokens.len() > 1 {
                let angle_str = token.strip_suffix("deg").unwrap_or(token);
                if let Ok(a) = angle_str.parse::<f32>() {
                    angle = a;
                } else {
                    return Err(ColorError::InvalidAngle(token.clone()));
                }
            } else {
                return Err(ColorError::InvalidFormat(token.clone()));
            }
        }

        if colors.is_empty() {
            return Err(ColorError::NoColors(input.to_string()));
        }

        if colors.len() > 1 {
            Ok(DrawingColor::Gradient(colors, angle))
        } else {
            Ok(DrawingColor::Solid(colors[0]))
        }
    }
}

fn tokenize(input: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current_token = String::new();
    let mut in_parens = 0;

    for c in input.chars() {
        if c == '(' {
            in_parens += 1;
            current_token.push(c);
        } else if c == ')' {
            in_parens -= 1;
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
    parse_rgba_hex(s)
        .or_else(|| parse_rgb_hex(s))
        .or_else(|| parse_hex(s))
}

fn parse_rgba_hex(s: &str) -> Option<Color> {
    if s.starts_with("rgba(") && s.ends_with(')') {
        let content = &s[5..s.len() - 1];
        if content.len() == 8 {
            let r = u8::from_str_radix(&content[0..2], 16).ok()?;
            let g = u8::from_str_radix(&content[2..4], 16).ok()?;
            let b = u8::from_str_radix(&content[4..6], 16).ok()?;
            let a = u8::from_str_radix(&content[6..8], 16).ok()?;
            return Some(Color::new(r, g, b, a));
        }
    }
    None
}

fn parse_rgb_hex(s: &str) -> Option<Color> {
    if s.starts_with("rgb(") && s.ends_with(')') {
        let content = &s[4..s.len() - 1];
        if content.len() == 6 {
            let r = u8::from_str_radix(&content[0..2], 16).ok()?;
            let g = u8::from_str_radix(&content[2..4], 16).ok()?;
            let b = u8::from_str_radix(&content[4..6], 16).ok()?;
            return Some(Color::new(r, g, b, 255));
        }
    }
    None
}

fn parse_hex(s: &str) -> Option<Color> {
    if s.starts_with('#') {
        if s.len() == 7 {
            let r = u8::from_str_radix(&s[1..3], 16).ok()?;
            let g = u8::from_str_radix(&s[3..5], 16).ok()?;
            let b = u8::from_str_radix(&s[5..7], 16).ok()?;
            return Some(Color::new(r, g, b, 255));
        } else if s.len() == 9 {
            let r = u8::from_str_radix(&s[1..3], 16).ok()?;
            let g = u8::from_str_radix(&s[3..5], 16).ok()?;
            let b = u8::from_str_radix(&s[5..7], 16).ok()?;
            let a = u8::from_str_radix(&s[7..9], 16).ok()?;
            return Some(Color::new(r, g, b, a));
        }
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
    fn test_parse_gradient() {
        let c = DrawingColor::parse("#ff0000 #00ff00 90deg").unwrap();
        if let DrawingColor::Gradient(colors, angle) = c {
            assert_eq!(colors.len(), 2);
            assert_eq!(colors[0], Color::new(255, 0, 0, 255));
            assert_eq!(colors[1], Color::new(0, 255, 0, 255));
            assert_eq!(angle, 90.0);
        } else {
            panic!("Expected gradient");
        }
    }

    #[test]
    fn test_parse_errors() {
        assert_eq!(DrawingColor::parse("  "), Err(ColorError::Empty));
        assert!(DrawingColor::parse("not-a-color").is_err());
        assert!(matches!(DrawingColor::parse("#ff0000 #00ff00 invalid"), Err(ColorError::InvalidAngle(_))));
    }

    #[test]
    fn test_tokenize() {
        let tokens = tokenize("rgb(255, 255, 255) #000");
        // tokenize doesn't split inside parens
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0], "rgb(255, 255, 255)");
    }
}
