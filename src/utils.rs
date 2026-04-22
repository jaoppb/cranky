use resvg::usvg;
use serde::{Deserialize, Deserializer};
use std::path::Path;
use thiserror::Error;
use tiny_skia::{Color, GradientStop, LinearGradient, Paint, Point, Rect, SpreadMode, Transform};

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
        let mut paint = Paint {
            anti_alias: true,
            ..Paint::default()
        };

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
        let mut paint = Paint {
            anti_alias: true,
            ..Paint::default()
        };

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

                if let Some(shader) =
                    LinearGradient::new(start, end, stops, SpreadMode::Pad, Transform::identity())
                {
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

pub fn rasterize_svg_icon_rgba(
    path: &Path,
    icon_size: u16,
    scale: f32,
) -> Option<image::RgbaImage> {
    let svg_data = std::fs::read(path).ok()?;
    let tree = usvg::Tree::from_data(&svg_data, &usvg::Options::default()).ok()?;
    let icon_px = ((icon_size as f32) * scale.max(1.0))
        .ceil()
        .max(icon_size as f32) as u32;
    let target = icon_px.max(1);
    let tree_size = tree.size();
    let sx = target as f32 / tree_size.width();
    let sy = target as f32 / tree_size.height();
    let fit_scale = sx.min(sy).max(0.001);
    let width = (tree_size.width() * fit_scale).ceil().max(1.0) as u32;
    let height = (tree_size.height() * fit_scale).ceil().max(1.0) as u32;
    let mut pixmap = tiny_skia::Pixmap::new(width, height)?;
    let transform = Transform::from_scale(fit_scale, fit_scale);
    let mut pixmap_mut = pixmap.as_mut();
    resvg::render(&tree, transform, &mut pixmap_mut);

    let mut rgba = image::RgbaImage::new(width, height);
    for (idx, pixel) in pixmap.data().chunks_exact(4).enumerate() {
        let a = pixel[3];
        let (r, g, b) = if a == 0 {
            (0, 0, 0)
        } else {
            let unpremul =
                |c: u8| -> u8 { (((c as u16 * 255) + (a as u16 / 2)) / a as u16).min(255) as u8 };
            (unpremul(pixel[0]), unpremul(pixel[1]), unpremul(pixel[2]))
        };
        let x = (idx as u32) % width;
        let y = (idx as u32) / width;
        rgba.put_pixel(x, y, image::Rgba([r, g, b, a]));
    }

    Some(rgba)
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
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_svg_path() -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "cranky-utils-test-{}-{}.svg",
            std::process::id(),
            nanos
        ))
    }

    #[test]
    fn test_parse_color_solid() {
        let white = ParsedColor::try_from("#ffffff").unwrap();
        assert_eq!(
            white,
            ParsedColor::Solid(Color::from_rgba8(255, 255, 255, 255))
        );

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
        assert!(matches!(
            ParsedColor::try_from("invalid"),
            Err(ColorParseError::InvalidColor(_))
        ));
        assert_eq!(ParsedColor::try_from(""), Err(ColorParseError::Empty));
        assert!(matches!(
            ParsedColor::try_from("rgb(zzzzzz)"),
            Err(ColorParseError::InvalidColor(_))
        ));
        assert!(matches!(
            ParsedColor::try_from("#12345"),
            Err(ColorParseError::InvalidColor(_))
        ));
        assert!(matches!(
            ParsedColor::try_from("rgb(ff0000) 45xx"),
            Err(ColorParseError::InvalidAngle(_))
        ));
    }

    #[test]
    fn test_parse_color_deserialize() {
        #[derive(Deserialize)]
        struct Wrapper {
            color: ParsedColor,
        }
        let wrapped: Wrapper = serde_json::from_str(r##"{"color":"#00ff00"}"##).unwrap();
        assert_eq!(
            wrapped.color,
            ParsedColor::Solid(Color::from_rgba8(0, 255, 0, 255))
        );
    }

    #[test]
    fn test_gradient_to_paint_builds_shader() {
        let color = ParsedColor::Gradient(
            vec![
                Color::from_rgba8(255, 0, 0, 255),
                Color::from_rgba8(0, 0, 255, 255),
            ],
            90.0,
        );
        let rect = Rect::from_xywh(0.0, 0.0, 20.0, 10.0).unwrap();
        let paint = color.to_paint(rect);
        assert!(paint.anti_alias);
    }

    #[test]
    fn test_rasterize_svg_icon_rgba_missing_file() {
        let missing = std::env::temp_dir().join("definitely-missing-cranky.svg");
        assert!(rasterize_svg_icon_rgba(&missing, 16, 1.0).is_none());
    }

    #[test]
    fn test_rasterize_svg_icon_rgba_success() {
        let path = temp_svg_path();
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24"><circle cx="12" cy="12" r="10" fill="#ff00ff"/></svg>"##;
        fs::write(&path, svg).unwrap();

        let rasterized = rasterize_svg_icon_rgba(&path, 16, 1.0);
        let _ = fs::remove_file(&path);

        assert!(rasterized.is_some());
        let image = rasterized.unwrap();
        assert!(image.width() > 0);
        assert!(image.height() > 0);
    }

    #[test]
    fn test_rasterize_svg_icon_rgba_invalid_svg() {
        let path = temp_svg_path();
        fs::write(&path, "this is not svg").unwrap();

        let rasterized = rasterize_svg_icon_rgba(&path, 16, 1.0);
        let _ = fs::remove_file(&path);

        assert!(rasterized.is_none());
    }
}
