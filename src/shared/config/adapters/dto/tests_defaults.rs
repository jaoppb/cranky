//! Tests that the margin and tooltip DTOs fall back the way `config.toml` documents when a field
//! is left out or does not parse.
use super::*;
use crate::shared::config::domain::{
    BorderRadius, BorderSize, FontFamily, FontSize, PaddingOffset, TooltipConfig,
};
use crate::shared::primitives::color::DrawingColor;

#[test]
fn margin_without_any_field_is_zero_on_every_side() {
    let domain = toml::from_str::<MarginConfigDto>("").unwrap().into_domain();
    assert_eq!(
        [domain.top(), domain.bottom(), domain.left(), domain.right()]
            .map(crate::shared::config::domain::MarginOffset::value),
        [0, 0, 0, 0]
    );
}

#[test]
fn margin_sides_fall_back_to_their_axis_then_to_zero() {
    let domain = toml::from_str::<MarginConfigDto>("vertical = 3\nright = 4")
        .unwrap()
        .into_domain();
    assert_eq!(
        [domain.top(), domain.bottom(), domain.left(), domain.right()]
            .map(crate::shared::config::domain::MarginOffset::value),
        [3, 3, 0, 4]
    );
}

#[test]
fn partial_margin_without_any_field_leaves_every_side_unset() {
    let domain = toml::from_str::<PartialMarginConfigDto>("")
        .unwrap()
        .into_domain();
    assert_eq!(
        [domain.top(), domain.bottom(), domain.left(), domain.right()],
        [None, None, None, None]
    );
}

#[test]
fn partial_margin_sides_fall_back_to_their_axis_then_stay_unset() {
    let domain = toml::from_str::<PartialMarginConfigDto>("vertical = 3\nright = 4")
        .unwrap()
        .into_domain();
    assert_eq!(
        [domain.top(), domain.bottom(), domain.left(), domain.right()]
            .map(|side| side.map(crate::shared::config::domain::MarginOffset::value)),
        [Some(3), Some(3), None, Some(4)]
    );
}

#[test]
fn margin_rejects_a_value_that_is_neither_a_number_nor_a_table() {
    let error = toml::from_str::<MarginConfigDto>("top = \"wide\"").unwrap_err();
    assert!(
        error
            .to_string()
            .contains("data did not match any variant of untagged enum MarginConfigDto"),
        "{error}"
    );
}

const FULL_TOOLTIP: &str = r##"
    background = "#102030"
    border_color = "#405060"
    text_color = "#708090"
    font = "Roboto"
    size = 14.0
    radius = 6.0
    border_width = 2.0
    padding = 12
"##;

#[test]
fn tooltip_carries_every_field_into_the_domain() {
    let domain = toml::from_str::<TooltipConfigDto>(FULL_TOOLTIP)
        .unwrap()
        .into_domain();
    let expected = TooltipConfig::default()
        .with_background(DrawingColor::parse("#102030").unwrap())
        .with_border_color(DrawingColor::parse("#405060").unwrap())
        .with_text_color(DrawingColor::parse("#708090").unwrap())
        .with_font(Some(FontFamily::new("Roboto".to_string())))
        .with_size(Some(FontSize::new(14.0)))
        .with_radius(BorderRadius::new(6.0))
        .with_border_width(BorderSize::new(2.0))
        .with_padding(PaddingOffset::new(12));
    assert_eq!(domain, expected);
}

#[test]
fn tooltip_keeps_the_default_colors_when_they_do_not_parse() {
    let toml_str = r##"
        background = "not-a-color"
        border_color = "#12"
        text_color = "rgb(1, 2)"
    "##;
    let domain = toml::from_str::<TooltipConfigDto>(toml_str)
        .unwrap()
        .into_domain();
    assert_eq!(domain, TooltipConfig::default());
}

#[test]
fn tooltip_without_any_field_is_the_default() {
    let domain = toml::from_str::<TooltipConfigDto>("")
        .unwrap()
        .into_domain();
    assert_eq!(domain, TooltipConfig::default());
}
