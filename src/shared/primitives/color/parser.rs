use super::color_struct::Color;

pub(super) fn tokenize(input: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current_token = String::new();
    let mut in_parens: u32 = 0;

    for character in input.chars() {
        if character == '(' {
            in_parens = in_parens.saturating_add(1);
            current_token.push(character);
        } else if character == ')' {
            in_parens = in_parens.saturating_sub(1);
            current_token.push(character);
        } else if character.is_whitespace() && in_parens == 0 {
            if !current_token.is_empty() {
                tokens.push(current_token.clone());
                current_token.clear();
            }
        } else {
            current_token.push(character);
        }
    }
    if !current_token.is_empty() {
        tokens.push(current_token);
    }
    tokens
}

pub(super) fn parse_single_color(input: &str) -> Option<Color> {
    parse_css_rgba(input)
        .or_else(|| parse_css_rgb(input))
        .or_else(|| parse_rgba_hex(input))
        .or_else(|| parse_rgb_hex(input))
        .or_else(|| parse_hex(input))
}

fn parse_rgba_hex(input: &str) -> Option<Color> {
    let content = input.strip_prefix("rgba(")?.strip_suffix(')')?;
    if content.len() == 8 {
        let red = u8::from_str_radix(content.get(0..2)?, 16).ok()?;
        let green = u8::from_str_radix(content.get(2..4)?, 16).ok()?;
        let blue = u8::from_str_radix(content.get(4..6)?, 16).ok()?;
        let alpha = u8::from_str_radix(content.get(6..8)?, 16).ok()?;
        return Some(Color::rgb(red, green, blue).with_alpha(alpha));
    }
    None
}

fn parse_rgb_hex(input: &str) -> Option<Color> {
    let content = input.strip_prefix("rgb(")?.strip_suffix(')')?;
    if content.len() == 6 {
        let red = u8::from_str_radix(content.get(0..2)?, 16).ok()?;
        let green = u8::from_str_radix(content.get(2..4)?, 16).ok()?;
        let blue = u8::from_str_radix(content.get(4..6)?, 16).ok()?;
        return Some(Color::rgb(red, green, blue));
    }
    None
}

fn parse_hex(input: &str) -> Option<Color> {
    let hex = input.strip_prefix('#')?;
    if hex.len() == 6 {
        let red = u8::from_str_radix(hex.get(0..2)?, 16).ok()?;
        let green = u8::from_str_radix(hex.get(2..4)?, 16).ok()?;
        let blue = u8::from_str_radix(hex.get(4..6)?, 16).ok()?;
        Some(Color::rgb(red, green, blue))
    } else if hex.len() == 8 {
        let red = u8::from_str_radix(hex.get(0..2)?, 16).ok()?;
        let green = u8::from_str_radix(hex.get(2..4)?, 16).ok()?;
        let blue = u8::from_str_radix(hex.get(4..6)?, 16).ok()?;
        let alpha = u8::from_str_radix(hex.get(6..8)?, 16).ok()?;
        Some(Color::rgb(red, green, blue).with_alpha(alpha))
    } else {
        None
    }
}

fn parse_css_rgb(input: &str) -> Option<Color> {
    let content = input.strip_prefix("rgb(")?.strip_suffix(')')?;
    let mut parts = content.split(',').map(str::trim);
    let red_str = parts.next()?;
    let green_str = parts.next()?;
    let blue_str = parts.next()?;
    if parts.next().is_none() {
        let red = red_str.parse::<u8>().ok()?;
        let green = green_str.parse::<u8>().ok()?;
        let blue = blue_str.parse::<u8>().ok()?;
        return Some(Color::rgb(red, green, blue));
    }
    None
}

fn parse_css_rgba(input: &str) -> Option<Color> {
    let content = input.strip_prefix("rgba(")?.strip_suffix(')')?;
    let mut parts = content.split(',').map(str::trim);
    let red_str = parts.next()?;
    let green_str = parts.next()?;
    let blue_str = parts.next()?;
    let alpha_str = parts.next()?;
    if parts.next().is_none() {
        let red = red_str.parse::<u8>().ok()?;
        let green = green_str.parse::<u8>().ok()?;
        let blue = blue_str.parse::<u8>().ok()?;
        let alpha_f = alpha_str.parse::<f32>().ok()?;
        let alpha =
            u8::try_from(crate::utils::f32_to_u32((alpha_f * 255.0).round())).unwrap_or(255);
        return Some(Color::rgb(red, green, blue).with_alpha(alpha));
    }
    None
}
