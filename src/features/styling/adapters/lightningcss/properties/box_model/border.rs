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

pub fn apply_border_properties(style: &mut ComputedStyle, prop: &Property) -> bool {
    match prop {
        Property::Border(border) => {
            apply_border(style, border);
            true
        }
        Property::BorderRadius(radius, _) => {
            let px = parse_length_str(
                &radius
                    .top_left
                    .0
                    .to_css_string(PrinterOptions::default())
                    .unwrap_or_default(),
            );
            style.set_border_radius(BorderRadius::new(px));
            true
        }
        Property::BorderWidth(width) => {
            let px = parse_length_str(
                &width
                    .top
                    .to_css_string(PrinterOptions::default())
                    .unwrap_or_default(),
            );
            style.set_border_size(BorderSize::new(px));
            true
        }
        Property::BorderColor(color) => {
            // A real `border-color` value is always one to four `<color>`s,
            // never a gradient — no string round-trip needed.
            if let Some(c) = convert_color(&color.top) {
                style.set_border_color(DrawingColor::Solid(c));
            }
            true
        }
        Property::BorderImageSource(image) => {
            if let Some(c) = gradient_from_image(image) {
                style.set_border_color(c);
            }
            true
        }
        Property::BorderImage(border_image, _) => {
            // Only the `source` longhand is implemented — `slice`/`width`/
            // `outset`/`repeat` (9-slice image compositing) have no
            // equivalent in this renderer and are silently left at their
            // defaults rather than attempted.
            if let Some(c) = gradient_from_image(&border_image.source) {
                style.set_border_color(c);
            }
            true
        }
        _ => false,
    }
}

pub fn apply_border(style: &mut ComputedStyle, border: &Border) {
    let px = parse_length_str(
        &border
            .width
            .to_css_string(PrinterOptions::default())
            .unwrap_or_default(),
    );
    style.set_border_size(BorderSize::new(px));
    if let Some(c) = convert_color(&border.color) {
        style.set_border_color(DrawingColor::Solid(c));
    }
}
