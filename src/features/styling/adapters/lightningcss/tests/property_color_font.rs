use super::declared;
use crate::features::styling::domain::Opacity;
use crate::shared::config::domain::{FontFamily, FontSize};
use crate::shared::primitives::color::{Color, DrawingColor};

fn solid(r: u8, g: u8, b: u8) -> DrawingColor {
    DrawingColor::Solid(Color::new(r, g, b, 255))
}

#[test]
fn each_colour_property_sets_its_own_field() {
    let style = declared("background-color: #010203; color: #040506; accent-color: #070809;");
    assert_eq!(style.background(), Some(&solid(1, 2, 3)));
    assert_eq!(style.color(), Some(&solid(4, 5, 6)));
    assert_eq!(style.accent_color(), Some(&solid(7, 8, 9)));
}

#[test]
fn background_shorthand_with_a_plain_colour_is_a_solid_fill() {
    let style = declared("background: #0a0b0c;");
    assert_eq!(style.background(), Some(&solid(10, 11, 12)));
}

#[test]
fn font_family_takes_the_first_family_without_its_quotes() {
    let style = declared("font-family: \"Fira Code\", monospace;");
    assert_eq!(
        style.font_family(),
        Some(&FontFamily::new("Fira Code".to_string()))
    );
}

#[test]
fn font_size_keywords_map_to_their_pixel_sizes() {
    let cases = [
        ("xx-small", 9.0),
        ("x-small", 10.0),
        ("small", 12.0),
        ("medium", 14.0),
        ("large", 18.0),
        ("x-large", 24.0),
        ("xx-large", 32.0),
        ("xxx-large", 48.0),
    ];
    for (keyword, px) in cases {
        let style = declared(&format!("font-size: {keyword};"));
        assert_eq!(style.font_size(), Some(FontSize::new(px)), "{keyword}");
    }
}

#[test]
fn font_size_in_pixels_is_taken_as_written() {
    assert_eq!(
        declared("font-size: 13px;").font_size(),
        Some(FontSize::new(13.0))
    );
}

#[test]
fn opacity_is_taken_as_written() {
    assert_eq!(
        declared("opacity: 0.25;").opacity(),
        Some(Opacity::new(0.25).unwrap())
    );
}
