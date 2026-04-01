use tiny_skia::Color;

pub fn parse_color(hex: &str) -> Color {
    // Simple hex parser (e.g., #RRGGBB)
    if hex.starts_with('#') && hex.len() == 7 {
        let r = u8::from_str_radix(&hex[1..3], 16).unwrap_or(0);
        let g = u8::from_str_radix(&hex[3..5], 16).unwrap_or(0);
        let b = u8::from_str_radix(&hex[5..7], 16).unwrap_or(0);
        Color::from_rgba8(r, g, b, 255)
    } else {
        Color::BLACK
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_color() {
        let white = parse_color("#ffffff");
        assert_eq!(white, Color::from_rgba8(255, 255, 255, 255));

        let black = parse_color("#000000");
        assert_eq!(black, Color::from_rgba8(0, 0, 0, 255));

        let red = parse_color("#ff0000");
        assert_eq!(red, Color::from_rgba8(255, 0, 0, 255));

        let invalid = parse_color("invalid");
        assert_eq!(invalid, Color::BLACK);
    }
}
