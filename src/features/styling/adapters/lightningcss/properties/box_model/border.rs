//! Applies `border`, `border-width`, `border-color`, `border-radius` and `border-image`
//! (gradient sources only).
use super::super::color_font::convert_color;
use super::super::gradient::gradient_from_image;
use super::utils::parse_length_str;
use crate::features::styling::domain::ComputedStyle;
use crate::shared::config::domain::{BorderRadius, BorderSize};
use crate::shared::primitives::color::DrawingColor;
use lightningcss::properties::Property;
use lightningcss::properties::border::Border;
use lightningcss::stylesheet::PrinterOptions;
use lightningcss::traits::ToCss;
use lightningcss::values::color::CssColor;
use lightningcss::values::image::Image;

pub(crate) fn apply_border_properties(style: &mut ComputedStyle, prop: &Property<'_>) -> bool {
    match prop {
        Property::Border(border) => apply_border(style, border),
        Property::BorderRadius(radius, _) => {
            style.set_border_radius(BorderRadius::new(css_pixels(&radius.top_left.0)));
        }
        Property::BorderWidth(width) => {
            style.set_border_size(BorderSize::new(css_pixels(&width.top)));
        }
        // A real `border-color` value is always one to four `<color>`s,
        // never a gradient — no string round-trip needed.
        Property::BorderColor(color) => apply_border_color(style, &color.top),
        Property::BorderImageSource(image) => apply_border_image(style, image),
        // Only the `source` longhand is implemented — `slice`/`width`/
        // `outset`/`repeat` (9-slice image compositing) have no
        // equivalent in this renderer and are silently left at their
        // defaults rather than attempted.
        Property::BorderImage(border_image, _) => apply_border_image(style, &border_image.source),
        _ => return false,
    }
    true
}

pub(crate) fn apply_border(style: &mut ComputedStyle, border: &Border) {
    style.set_border_size(BorderSize::new(css_pixels(&border.width)));
    apply_border_color(style, &border.color);
}

/// Sets a solid border colour when `color` converts to one, and leaves the declared one otherwise.
fn apply_border_color(style: &mut ComputedStyle, color: &CssColor) {
    if let Some(solid) = convert_color(color) {
        style.set_border_color(DrawingColor::Solid(solid));
    }
}

/// Sets the border colour from a gradient image, and leaves the declared one for any other image.
fn apply_border_image(style: &mut ComputedStyle, image: &Image<'_>) {
    if let Some(gradient) = gradient_from_image(image) {
        style.set_border_color(gradient);
    }
}

/// The pixels a length serializes to, `em`/`rem` taken as 16px and anything unreadable as 0.
fn css_pixels(length: &impl ToCss) -> f32 {
    parse_length_str(
        &length
            .to_css_string(PrinterOptions::default())
            .unwrap_or_default(),
    )
}
