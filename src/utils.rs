use tiny_skia::{Color, Paint, Rect, LinearGradient, GradientStop, Point, SpreadMode, Transform};
use thiserror::Error;
use serde::{Deserialize, Deserializer};

#[derive(Debug, Error, PartialEq)]
pub enum ColorParseError {
    #[error("empty color string")]
    Empty,
    #[error("no colors found")]
    NoColors,
    #[error("invalid color: {0}")]
    InvalidColor(String),
    #[error("invalid angle: {0}")]
    InvalidAngle(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ParsedColor {
    Solid(Color),
    Gradient(Vec<Color>, f32),
}

impl<'de> Deserialize<'de> for ParsedColor {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        ParsedColor::try_from(s.as_str()).map_err(serde::de::Error::custom)
    }
}

impl From<&ParsedColor> for Paint<'static> {
    fn from(color: &ParsedColor) -> Self {
        let mut paint = Paint::default();
        paint.anti_alias = true;
        match color {
            ParsedColor::Solid(c) => paint.set_color(*c),
            ParsedColor::Gradient(colors, _) => {
                if let Some(&c) = colors.first() {
                    paint.set_color(c);
                }
            }
        }
        paint
    }
}

impl ParsedColor {
    pub fn to_paint(&self, rect: Rect) -> Paint<'static> {
        let mut paint = Paint::default();
        paint.anti_alias = true;

        match self {
            ParsedColor::Solid(c) => {
                paint.set_color(*c);
            }
            ParsedColor::Gradient(colors, angle) => {
                let stops: Vec<GradientStop> = colors
                    .iter()
                    .enumerate()
                    .map(|(i, &c)| {
                        GradientStop::new(i as f32 / (colors.len() - 1).max(1) as f32, c)
                    })
                    .collect();

                let angle_rad = angle.to_radians();
                let center_x = rect.left() + rect.width() / 2.0;
                let center_y = rect.top() + rect.height() / 2.0;

                let distance = (rect.width() / 2.0 * angle_rad.cos()).abs() 
                             + (rect.height() / 2.0 * angle_rad.sin()).abs();

                let x_offset = angle_rad.cos() * distance;
                let y_offset = angle_rad.sin() * distance;

                let start = Point::from_xy(center_x - x_offset, center_y - y_offset);
                let end = Point::from_xy(center_x + x_offset, center_y + y_offset);

                if let Some(shader) = LinearGradient::new(
                    start,
                    end,
                    stops,
                    SpreadMode::Pad,
                    Transform::identity(),
                ) {
                    paint.shader = shader;
                } else if let Some(&c) = colors.first() {
                    paint.set_color(c);
                }
            }
        }
        paint
    }
}

impl TryFrom<&str> for ParsedColor {
    type Error = ColorParseError;

    fn try_from(input: &str) -> Result<Self, Self::Error> {
        let input = input.trim();
        if input.is_empty() {
            return Err(ColorParseError::Empty);
        }

        let tokens = tokenize(input);
        if tokens.is_empty() {
            return Err(ColorParseError::NoColors);
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
                    return Err(ColorParseError::InvalidAngle(token.clone()));
                }
            } else {
                return Err(ColorParseError::InvalidColor(token.clone()));
            }
        }

        if colors.is_empty() {
            return Err(ColorParseError::NoColors);
        }

        if colors.len() > 1 {
            Ok(ParsedColor::Gradient(colors, angle))
        } else {
            Ok(ParsedColor::Solid(colors[0]))
        }
    }
}

impl TryFrom<String> for ParsedColor {
    type Error = ColorParseError;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::try_from(s.as_str())
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
            return Some(Color::from_rgba8(r, g, b, a));
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
            return Some(Color::from_rgba8(r, g, b, 255));
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
            return Some(Color::from_rgba8(r, g, b, 255));
        } else if s.len() == 9 {
            let r = u8::from_str_radix(&s[1..3], 16).ok()?;
            let g = u8::from_str_radix(&s[3..5], 16).ok()?;
            let b = u8::from_str_radix(&s[5..7], 16).ok()?;
            let a = u8::from_str_radix(&s[7..9], 16).ok()?;
            return Some(Color::from_rgba8(r, g, b, a));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_color_solid() {
        let white = ParsedColor::try_from("#ffffff").unwrap();
        assert_eq!(white, ParsedColor::Solid(Color::from_rgba8(255, 255, 255, 255)));

        let black = ParsedColor::try_from("rgb(000000)").unwrap();
        assert_eq!(black, ParsedColor::Solid(Color::from_rgba8(0, 0, 0, 255)));

        let red = ParsedColor::try_from("rgba(ff0000ff)").unwrap();
        assert_eq!(red, ParsedColor::Solid(Color::from_rgba8(255, 0, 0, 255)));
    }

    #[test]
    fn test_parse_color_gradient() {
        let grad = ParsedColor::try_from("rgba(ff0000ff) rgba(0000ffff) 45deg").unwrap();
        if let ParsedColor::Gradient(colors, angle) = grad {
            assert_eq!(colors.len(), 2);
            assert_eq!(colors[0], Color::from_rgba8(255, 0, 0, 255));
            assert_eq!(colors[1], Color::from_rgba8(0, 0, 255, 255));
            assert_eq!(angle, 45.0);
        } else {
            panic!("Expected Gradient");
        }

        let multi = ParsedColor::try_from("rgb(ff0000) rgb(00ff00) rgb(0000ff)").unwrap();
        if let ParsedColor::Gradient(colors, angle) = multi {
            assert_eq!(colors.len(), 3);
            assert_eq!(angle, 0.0);
        } else {
            panic!("Expected Gradient");
        }
    }

    #[test]
    fn test_parse_color_invalid() {
        assert!(matches!(ParsedColor::try_from("invalid"), Err(ColorParseError::InvalidColor(_))));
        assert_eq!(ParsedColor::try_from(""), Err(ColorParseError::Empty));
    }
}
