//! Converts a `linear-gradient()` `border-image` source into a gradient drawing colour.
use super::color_font::convert_color;
use crate::shared::primitives::color::{Color, DrawingColor};
use lightningcss::values::gradient::{Gradient, GradientItem, LineDirection, LinearGradient};
use lightningcss::values::image::Image;

/// Extracts a `DrawingColor` from a `border-image-source`/`border-image` value.
///
/// Only the one CSS shape this renderer actually paints — a directional
/// two-or-more-stop `linear-gradient()` — is supported. Everything else
/// `border-image` grammar allows (`url()`, `image-set()`, radial/conic
/// gradients, keyword directions like `to top`) has no equivalent in
/// `DrawingColor` and is rejected with a warn log rather than silently
/// mis-rendered — the same policy this codebase applies to every other
/// unrenderable CSS construct.
#[must_use]
pub(crate) fn gradient_from_image(image: &Image<'_>) -> Option<DrawingColor> {
    let Image::Gradient(gradient) = image else {
        if !matches!(image, Image::None) {
            log_unsupported_image(image);
        }
        return None;
    };
    match gradient.as_ref() {
        Gradient::Linear(linear) | Gradient::RepeatingLinear(linear) => {
            linear_gradient_colors(linear)
        }
        other => {
            log_unsupported_gradient(other);
            None
        }
    }
}

/// Warns that a `border-image` source other than a gradient is ignored.
///
/// Kept out of line to keep its caller under `cognitive_complexity`.
fn log_unsupported_image(image: &Image<'_>) {
    tracing::warn!(
        ?image,
        "border-image only supports linear-gradient(); ignoring"
    );
}

/// Warns that a gradient other than a linear one is ignored.
///
/// Kept out of line to keep its caller under `cognitive_complexity`.
fn log_unsupported_gradient(other: &Gradient) {
    tracing::warn!(?other, "only linear-gradient() border images are supported");
}

fn linear_gradient_colors(gradient: &LinearGradient) -> Option<DrawingColor> {
    let LineDirection::Angle(angle) = &gradient.direction else {
        log_keyword_direction(&gradient.direction);
        return None;
    };
    let colors = stop_colors(gradient);
    match colors.len() {
        0 => None,
        1 => colors.into_iter().next().map(DrawingColor::Solid),
        _ => Some(DrawingColor::Gradient(colors, angle.to_degrees())),
    }
}

/// Warns that a gradient whose direction is not an angle is ignored.
///
/// Kept out of line to keep its caller under `cognitive_complexity`.
fn log_keyword_direction(direction: &LineDirection) {
    tracing::warn!(
        ?direction,
        "border-image linear-gradient() direction must be an angle; ignoring"
    );
}

/// The colours of the gradient's stops that convert, in order, skipping its hints.
fn stop_colors(gradient: &LinearGradient) -> Vec<Color> {
    gradient
        .items
        .iter()
        .filter_map(|item| match item {
            GradientItem::ColorStop(stop) => convert_color(&stop.color),
            GradientItem::Hint(_) => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use lightningcss::properties::Property;
    use lightningcss::rules::CssRule;
    use lightningcss::stylesheet::{ParserOptions, StyleSheet};

    fn border_image_source(css: &str) -> Image<'_> {
        let sheet = StyleSheet::parse(css, ParserOptions::default()).unwrap();
        let CssRule::Style(style_rule) = sheet.rules.0.first().unwrap() else {
            panic!("expected a style rule");
        };
        let Property::BorderImageSource(image) =
            style_rule.declarations.declarations.first().unwrap()
        else {
            panic!("expected a border-image-source declaration");
        };
        image.clone()
    }

    #[test]
    fn test_linear_gradient_two_stops_extracts_colors_and_angle() {
        let image = border_image_source(
            "bar { border-image-source: linear-gradient(45deg, #7aa2f7, #bb9af7); }",
        );
        let Some(DrawingColor::Gradient(colors, angle)) = gradient_from_image(&image) else {
            panic!("expected a gradient");
        };
        assert_eq!(colors.len(), 2);
        assert!((angle - 45.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_keyword_direction_is_rejected() {
        let image = border_image_source(
            "bar { border-image-source: linear-gradient(to top, #7aa2f7, #bb9af7); }",
        );
        assert!(gradient_from_image(&image).is_none());
    }

    #[test]
    fn test_none_image_is_not_a_gradient() {
        let image = border_image_source("bar { border-image-source: none; }");
        assert!(gradient_from_image(&image).is_none());
    }

    #[test]
    fn test_url_and_radial_images_are_not_gradients() {
        let image = border_image_source("bar { border-image-source: url(edge.png); }");
        assert!(gradient_from_image(&image).is_none());
        let image =
            border_image_source("bar { border-image-source: radial-gradient(#7aa2f7, #bb9af7); }");
        assert!(gradient_from_image(&image).is_none());
    }

    #[test]
    fn test_repeating_linear_gradient_skips_hints_and_converts_the_angle() {
        let image = border_image_source(
            "bar { border-image-source: \
             repeating-linear-gradient(0.5turn, #010203, 40%, #040506 50%, #070809); }",
        );
        assert_eq!(
            gradient_from_image(&image),
            Some(DrawingColor::Gradient(
                vec![
                    crate::shared::primitives::color::Color::rgb(1, 2, 3),
                    crate::shared::primitives::color::Color::rgb(4, 5, 6),
                    crate::shared::primitives::color::Color::rgb(7, 8, 9),
                ],
                180.0
            ))
        );
    }

    #[test]
    fn test_single_convertible_stop_is_a_solid_colour_and_none_is_nothing() {
        let image = border_image_source(
            "bar { border-image-source: linear-gradient(10deg, currentColor, #040506); }",
        );
        assert_eq!(
            gradient_from_image(&image),
            Some(DrawingColor::Solid(
                crate::shared::primitives::color::Color::rgb(4, 5, 6)
            ))
        );
        let image = border_image_source(
            "bar { border-image-source: linear-gradient(10deg, currentColor, currentColor); }",
        );
        assert!(gradient_from_image(&image).is_none());
    }
}
