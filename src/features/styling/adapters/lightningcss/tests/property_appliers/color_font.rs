use super::applied;
use crate::features::styling::adapters::lightningcss::properties::color_font::apply_color_and_font;
use crate::features::styling::domain::Opacity;
use crate::shared::config::domain::{FontFamily, FontSize};
use crate::shared::primitives::TextAlign;
use crate::shared::primitives::color::{Color, DrawingColor};

fn solid(red: u8, green: u8, blue: u8) -> DrawingColor {
    DrawingColor::Solid(Color::rgb(red, green, blue))
}

#[test]
fn each_colour_property_is_handled_into_its_own_field() {
    let (style, handled) = applied(
        apply_color_and_font,
        "background-color: #010203; color: #040506; accent-color: #070809;",
    );
    assert_eq!(handled, [true; 3]);
    assert_eq!(style.background(), Some(&solid(1, 2, 3)));
    assert_eq!(style.color(), Some(&solid(4, 5, 6)));
    assert_eq!(style.accent_color(), Some(&solid(7, 8, 9)));
}

#[test]
fn colours_that_do_not_convert_are_handled_but_not_set() {
    let (style, handled) = applied(
        apply_color_and_font,
        "background-color: currentColor; color: currentColor; accent-color: auto;",
    );
    assert_eq!(handled, [true; 3]);
    assert_eq!(style.background(), None);
    assert_eq!(style.color(), None);
    assert_eq!(style.accent_color(), None);
}

#[test]
fn font_family_strips_either_kind_of_quote_from_the_first_family() {
    let (style, handled) = applied(apply_color_and_font, "font-family: 'Iosevka Term', serif;");
    assert_eq!(handled, [true]);
    assert_eq!(
        style.font_family(),
        Some(&FontFamily::new("Iosevka Term".to_string()))
    );
    let (style, _) = applied(apply_color_and_font, "font-family: monospace, serif;");
    assert_eq!(
        style.font_family(),
        Some(&FontFamily::new("monospace".to_string()))
    );
}

#[test]
fn background_layers_fall_back_to_the_first_layer_colour_only() {
    let (style, handled) = applied(
        apply_color_and_font,
        "background: url(top.png), url(bottom.png) #0a0b0c;",
    );
    assert_eq!(handled, [true]);
    assert_eq!(
        style.background(),
        Some(&DrawingColor::Solid(Color::rgb(0, 0, 0).with_alpha(0)))
    );
}

#[test]
fn text_align_keeps_only_the_three_painted_keywords() {
    for (keyword, expected) in [
        ("left", Some(TextAlign::Left)),
        ("center", Some(TextAlign::Center)),
        ("right", Some(TextAlign::Right)),
        ("justify", None),
    ] {
        let (style, handled) = applied(apply_color_and_font, &format!("text-align: {keyword};"));
        assert_eq!(handled, [true], "{keyword}");
        assert_eq!(style.text_align(), expected, "{keyword}");
    }
}

#[test]
fn font_size_lengths_keywords_and_relative_sizes() {
    let (style, handled) = applied(apply_color_and_font, "font-size: 2em;");
    assert_eq!(handled, [true]);
    assert_eq!(style.font_size(), Some(FontSize::new(32.0)));
    let (style, _) = applied(apply_color_and_font, "font-size: large;");
    assert_eq!(style.font_size(), Some(FontSize::new(18.0)));
    let (style, handled) = applied(apply_color_and_font, "font-size: larger;");
    assert_eq!(handled, [true]);
    assert_eq!(style.font_size(), None);
}

#[test]
fn background_shorthand_falls_back_to_its_first_layer_colour() {
    let (style, handled) = applied(apply_color_and_font, "background: url(tile.png) #0a0b0c;");
    assert_eq!(handled, [true]);
    assert_eq!(style.background(), Some(&solid(10, 11, 12)));
}

#[test]
fn background_shorthand_without_a_colour_takes_the_transparent_initial_one() {
    let (style, handled) = applied(apply_color_and_font, "background: url(tile.png);");
    assert_eq!(handled, [true]);
    assert_eq!(
        style.background(),
        Some(&DrawingColor::Solid(Color::rgb(0, 0, 0).with_alpha(0)))
    );
}

#[test]
fn opacity_out_of_range_is_handled_but_not_set() {
    let (style, handled) = applied(apply_color_and_font, "opacity: 0.5;");
    assert_eq!(handled, [true]);
    assert_eq!(style.opacity(), Some(Opacity::new(0.5).unwrap()));
    let (style, handled) = applied(apply_color_and_font, "opacity: 1.5;");
    assert_eq!(handled, [true]);
    assert_eq!(style.opacity(), None);
}

#[test]
fn colour_and_font_applier_declines_other_properties() {
    let (_, handled) = applied(
        apply_color_and_font,
        "border-color: #010203; width: 1px; font-weight: bold;",
    );
    assert_eq!(handled, [false, false, false]);
}
