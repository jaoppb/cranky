use super::applied;
use crate::features::styling::adapters::lightningcss::properties::box_model::{
    apply_border_properties, apply_sizing_properties,
};
use crate::features::styling::domain::CssLength;
use crate::shared::config::domain::{BorderRadius, BorderSize};
use crate::shared::primitives::color::{Color, DrawingColor};

fn solid(red: u8, green: u8, blue: u8) -> DrawingColor {
    DrawingColor::Solid(Color::rgb(red, green, blue))
}

#[test]
fn border_shorthand_sets_size_and_colour() {
    let (style, handled) = applied(apply_border_properties, "border: 3px solid #010203;");
    assert_eq!(handled, [true]);
    assert_eq!(style.border_size(), Some(BorderSize::new(3.0)));
    assert_eq!(style.border_color(), Some(&solid(1, 2, 3)));
}

#[test]
fn border_shorthand_without_a_convertible_colour_keeps_only_the_size() {
    let (style, handled) = applied(apply_border_properties, "border: 2px solid currentColor;");
    assert_eq!(handled, [true]);
    assert_eq!(style.border_size(), Some(BorderSize::new(2.0)));
    assert_eq!(style.border_color(), None);
}

#[test]
fn border_radius_takes_the_top_left_corner() {
    let (style, handled) = applied(apply_border_properties, "border-radius: 5px 9px 11px 13px;");
    assert_eq!(handled, [true]);
    assert_eq!(style.border_radius(), Some(BorderRadius::new(5.0)));
    assert_eq!(style.border_size(), None);
}

#[test]
fn border_width_takes_the_top_side() {
    let (style, handled) = applied(apply_border_properties, "border-width: 4px 8px;");
    assert_eq!(handled, [true]);
    assert_eq!(style.border_size(), Some(BorderSize::new(4.0)));
    assert_eq!(style.border_radius(), None);
}

#[test]
fn border_color_takes_the_top_side() {
    let (style, handled) = applied(apply_border_properties, "border-color: #040506 #070809;");
    assert_eq!(handled, [true]);
    assert_eq!(style.border_color(), Some(&solid(4, 5, 6)));
}

#[test]
fn border_color_that_does_not_convert_is_handled_but_not_set() {
    let (style, handled) = applied(apply_border_properties, "border-color: currentColor;");
    assert_eq!(handled, [true]);
    assert_eq!(style.border_color(), None);
}

#[test]
fn border_image_source_gradient_becomes_the_border_colour() {
    let (style, handled) = applied(
        apply_border_properties,
        "border-image-source: linear-gradient(90deg, #010203, #040506);",
    );
    assert_eq!(handled, [true]);
    assert_eq!(
        style.border_color(),
        Some(&DrawingColor::Gradient(
            vec![Color::rgb(1, 2, 3), Color::rgb(4, 5, 6)],
            90.0
        ))
    );
}

#[test]
fn border_image_shorthand_uses_only_its_gradient_source() {
    let (style, handled) = applied(
        apply_border_properties,
        "border-image: linear-gradient(30deg, #070809, #0a0b0c) 1;",
    );
    assert_eq!(handled, [true]);
    assert_eq!(
        style.border_color(),
        Some(&DrawingColor::Gradient(
            vec![Color::rgb(7, 8, 9), Color::rgb(10, 11, 12)],
            30.0
        ))
    );
}

#[test]
fn border_images_without_a_gradient_are_handled_but_not_set() {
    let (style, handled) = applied(
        apply_border_properties,
        "border-image-source: none; border-image: url(edge.png) 2;",
    );
    assert_eq!(handled, [true, true]);
    assert_eq!(style.border_color(), None);
}

#[test]
fn border_applier_declines_other_properties() {
    let (_, handled) = applied(
        apply_border_properties,
        "color: #010203; width: 4px; padding: 1px; border-style: solid;",
    );
    assert_eq!(handled, [false, false, false, false]);
}

#[test]
fn each_sizing_property_is_handled_into_its_own_field() {
    let (style, handled) = applied(
        apply_sizing_properties,
        "width: 11px; height: 12%; min-width: 13px; max-width: 14px; \
         min-height: 15px; max-height: 16px;",
    );
    assert_eq!(handled, [true; 6]);
    assert_eq!(style.width(), Some(CssLength::Pixels(11.0)));
    assert_eq!(style.height(), Some(CssLength::Percent(12.0)));
    assert_eq!(style.min_width(), Some(CssLength::Pixels(13.0)));
    assert_eq!(style.max_width(), Some(CssLength::Pixels(14.0)));
    assert_eq!(style.min_height(), Some(CssLength::Pixels(15.0)));
    assert_eq!(style.max_height(), Some(CssLength::Pixels(16.0)));
}

#[test]
fn sizing_values_without_a_length_are_handled_but_not_set() {
    let (style, handled) = applied(
        apply_sizing_properties,
        "max-width: none; max-height: none;",
    );
    assert_eq!(handled, [true, true]);
    assert_eq!(style.max_width(), None);
    assert_eq!(style.max_height(), None);
}

#[test]
fn sizing_auto_is_kept_on_every_axis() {
    let (style, _) = applied(
        apply_sizing_properties,
        "width: auto; min-width: auto; min-height: auto;",
    );
    assert_eq!(style.width(), Some(CssLength::Auto));
    assert_eq!(style.min_width(), Some(CssLength::Auto));
    assert_eq!(style.min_height(), Some(CssLength::Auto));
}

#[test]
fn sizing_applier_declines_other_properties() {
    let (_, handled) = applied(
        apply_sizing_properties,
        "border-width: 1px; flex-basis: 3px;",
    );
    assert_eq!(handled, [false, false]);
}
