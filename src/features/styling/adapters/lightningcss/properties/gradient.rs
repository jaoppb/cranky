use super::color_font::convert_color;
use crate::shared::primitives::color::DrawingColor;
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
pub fn gradient_from_image(image: &Image) -> Option<DrawingColor> {
    let Image::Gradient(gradient) = image else {
        if !matches!(image, Image::None) {
            tracing::warn!(
                ?image,
                "border-image only supports linear-gradient(); ignoring"
            );
        }
        return None;
    };
    match gradient.as_ref() {
        Gradient::Linear(lg) | Gradient::RepeatingLinear(lg) => linear_gradient_colors(lg),
        other => {
            tracing::warn!(?other, "only linear-gradient() border images are supported");
            None
        }
    }
}

fn linear_gradient_colors(lg: &LinearGradient) -> Option<DrawingColor> {
    let LineDirection::Angle(angle) = &lg.direction else {
        tracing::warn!(
            direction = ?lg.direction,
            "border-image linear-gradient() direction must be an angle; ignoring"
        );
        return None;
    };

    let colors: Vec<_> = lg
        .items
        .iter()
        .filter_map(|item| match item {
            GradientItem::ColorStop(stop) => convert_color(&stop.color),
            GradientItem::Hint(_) => None,
        })
        .collect();

    match colors.len() {
        0 => None,
        1 => colors.into_iter().next().map(DrawingColor::Solid),
        _ => Some(DrawingColor::Gradient(colors, angle.to_degrees())),
    }
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
}
