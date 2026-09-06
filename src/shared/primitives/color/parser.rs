use super::color_struct::Color;

pub fn tokenize(input: &str) -> Vec<String> {
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

pub fn parse_single_color(s: &str) -> Option<Color> {
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
