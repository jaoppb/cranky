use super::utils::parse_size_str;
use crate::features::styling::domain::ComputedStyle;
use lightningcss::properties::Property;
use lightningcss::stylesheet::PrinterOptions;
use lightningcss::traits::ToCss;

pub fn apply_sizing_properties(style: &mut ComputedStyle, prop: &Property) -> bool {
    match prop {
        Property::Width(size) => {
            let s = size
                .to_css_string(PrinterOptions::default())
                .unwrap_or_default();
            if let Some(w) = parse_size_str(&s) {
                style.set_width(w);
            }
            true
        }
        Property::Height(size) => {
            let s = size
                .to_css_string(PrinterOptions::default())
                .unwrap_or_default();
            if let Some(h) = parse_size_str(&s) {
                style.set_height(h);
            }
            true
        }
        Property::MinWidth(size) => {
            let s = size
                .to_css_string(PrinterOptions::default())
                .unwrap_or_default();
            if let Some(mw) = parse_size_str(&s) {
                style.set_min_width(mw);
            }
            true
        }
        Property::MaxWidth(size) => {
            let s = size
                .to_css_string(PrinterOptions::default())
                .unwrap_or_default();
            if let Some(mw) = parse_size_str(&s) {
                style.set_max_width(mw);
            }
            true
        }
        Property::MinHeight(size) => {
            let s = size
                .to_css_string(PrinterOptions::default())
                .unwrap_or_default();
            if let Some(mh) = parse_size_str(&s) {
                style.set_min_height(mh);
            }
            true
        }
        Property::MaxHeight(size) => {
            let s = size
                .to_css_string(PrinterOptions::default())
                .unwrap_or_default();
            if let Some(mh) = parse_size_str(&s) {
                style.set_max_height(mh);
            }
            true
        }
        _ => false,
    }
}
