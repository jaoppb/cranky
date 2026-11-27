//! Applies `width`, `height` and their `min-`/`max-` variants.
use super::utils::parse_size_str;
use crate::features::styling::domain::{ComputedStyle, CssLength};
use lightningcss::properties::Property;
use lightningcss::stylesheet::PrinterOptions;
use lightningcss::traits::ToCss;

pub(crate) fn apply_sizing_properties(style: &mut ComputedStyle, prop: &Property<'_>) -> bool {
    match prop {
        Property::Width(size) => set_size(style, size, ComputedStyle::set_width),
        Property::Height(size) => set_size(style, size, ComputedStyle::set_height),
        Property::MinWidth(size) => set_size(style, size, ComputedStyle::set_min_width),
        Property::MaxWidth(size) => set_size(style, size, ComputedStyle::set_max_width),
        Property::MinHeight(size) => set_size(style, size, ComputedStyle::set_min_height),
        Property::MaxHeight(size) => set_size(style, size, ComputedStyle::set_max_height),
        _ => return false,
    }
    true
}

/// Writes `size` through `setter` when it serializes to a length, and leaves the field otherwise
/// (`none`).
fn set_size(
    style: &mut ComputedStyle,
    size: &impl ToCss,
    setter: fn(&mut ComputedStyle, CssLength),
) {
    let css_text = size
        .to_css_string(PrinterOptions::default())
        .unwrap_or_default();
    if let Some(length) = parse_size_str(&css_text) {
        setter(style, length);
    }
}
