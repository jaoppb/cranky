use super::super::color_font::convert_color;
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
            let s = color
                .to_css_string(PrinterOptions::default())
                .unwrap_or_default();
            if let Ok(c) = DrawingColor::parse(s.trim()) {
                style.set_border_color(c);
            } else if let Some(c) = convert_color(&color.top) {
                style.set_border_color(DrawingColor::Solid(c));
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
    let s = border
        .color
        .to_css_string(PrinterOptions::default())
        .unwrap_or_default();
    if let Ok(c) = DrawingColor::parse(s.trim()) {
        style.set_border_color(c);
    } else if let Some(c) = convert_color(&border.color) {
        style.set_border_color(DrawingColor::Solid(c));
    }
}
