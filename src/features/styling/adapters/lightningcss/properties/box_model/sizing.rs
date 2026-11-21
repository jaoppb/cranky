//! Applies `width`, `height` and their `min-`/`max-` variants.
use super::utils::parse_size_str;
use crate::features::styling::domain::ComputedStyle;
use lightningcss::properties::Property;
use lightningcss::stylesheet::PrinterOptions;
use lightningcss::traits::ToCss;

pub(crate) fn apply_sizing_properties(style: &mut ComputedStyle, prop: &Property<'_>) -> bool {
    match prop {
        Property::Width(size) => {
            let css_text = size
                .to_css_string(PrinterOptions::default())
                .unwrap_or_default();
            if let Some(width) = parse_size_str(&css_text) {
                style.set_width(width);
            }
            true
        }
        Property::Height(size) => {
            let css_text = size
                .to_css_string(PrinterOptions::default())
                .unwrap_or_default();
            if let Some(height) = parse_size_str(&css_text) {
                style.set_height(height);
            }
            true
        }
        Property::MinWidth(size) => {
            let css_text = size
                .to_css_string(PrinterOptions::default())
                .unwrap_or_default();
            if let Some(min_width) = parse_size_str(&css_text) {
                style.set_min_width(min_width);
            }
            true
        }
        Property::MaxWidth(size) => {
            let css_text = size
                .to_css_string(PrinterOptions::default())
                .unwrap_or_default();
            if let Some(max_width) = parse_size_str(&css_text) {
                style.set_max_width(max_width);
            }
            true
        }
        Property::MinHeight(size) => {
            let css_text = size
                .to_css_string(PrinterOptions::default())
                .unwrap_or_default();
            if let Some(min_height) = parse_size_str(&css_text) {
                style.set_min_height(min_height);
            }
            true
        }
        Property::MaxHeight(size) => {
            let css_text = size
                .to_css_string(PrinterOptions::default())
                .unwrap_or_default();
            if let Some(max_height) = parse_size_str(&css_text) {
                style.set_max_height(max_height);
            }
            true
        }
        _ => false,
    }
}
