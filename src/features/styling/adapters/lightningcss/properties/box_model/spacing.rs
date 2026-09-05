use super::utils::length_to_f64;
use crate::features::layout_engine::domain::BoxMargin;
use crate::features::styling::domain::ComputedStyle;
use lightningcss::properties::Property;

pub fn apply_padding_properties(style: &mut ComputedStyle, prop: &Property) -> bool {
    match prop {
        Property::Padding(padding) => {
            let top = length_to_f64(&padding.top);
            let right = length_to_f64(&padding.right);
            let bottom = length_to_f64(&padding.bottom);
            let left = length_to_f64(&padding.left);
            style.set_padding(BoxMargin::new(top, bottom, left, right));
            true
        }
        Property::PaddingTop(len) => {
            let top = length_to_f64(len);
            let current = style.padding().cloned().unwrap_or_default();
            style.set_padding(BoxMargin::new(
                top,
                current.bottom(),
                current.left(),
                current.right(),
            ));
            true
        }
        Property::PaddingRight(len) => {
            let right = length_to_f64(len);
            let current = style.padding().cloned().unwrap_or_default();
            style.set_padding(BoxMargin::new(
                current.top(),
                current.bottom(),
                current.left(),
                right,
            ));
            true
        }
        Property::PaddingBottom(len) => {
            let bottom = length_to_f64(len);
            let current = style.padding().cloned().unwrap_or_default();
            style.set_padding(BoxMargin::new(
                current.top(),
                bottom,
                current.left(),
                current.right(),
            ));
            true
        }
        Property::PaddingLeft(len) => {
            let left = length_to_f64(len);
            let current = style.padding().cloned().unwrap_or_default();
            style.set_padding(BoxMargin::new(
                current.top(),
                current.bottom(),
                left,
                current.right(),
            ));
            true
        }
        _ => false,
    }
}

pub fn apply_margin_properties(style: &mut ComputedStyle, prop: &Property) -> bool {
    match prop {
        Property::Margin(margin) => {
            let top = length_to_f64(&margin.top);
            let right = length_to_f64(&margin.right);
            let bottom = length_to_f64(&margin.bottom);
            let left = length_to_f64(&margin.left);
            style.set_margin(BoxMargin::new(top, bottom, left, right));
            true
        }
        Property::MarginTop(len) => {
            let top = length_to_f64(len);
            let current = style.margin().cloned().unwrap_or_default();
            style.set_margin(BoxMargin::new(
                top,
                current.bottom(),
                current.left(),
                current.right(),
            ));
            true
        }
        Property::MarginRight(len) => {
            let right = length_to_f64(len);
            let current = style.margin().cloned().unwrap_or_default();
            style.set_margin(BoxMargin::new(
                current.top(),
                current.bottom(),
                current.left(),
                right,
            ));
            true
        }
        Property::MarginBottom(len) => {
            let bottom = length_to_f64(len);
            let current = style.margin().cloned().unwrap_or_default();
            style.set_margin(BoxMargin::new(
                current.top(),
                bottom,
                current.left(),
                current.right(),
            ));
            true
        }
        Property::MarginLeft(len) => {
            let left = length_to_f64(len);
            let current = style.margin().cloned().unwrap_or_default();
            style.set_margin(BoxMargin::new(
                current.top(),
                current.bottom(),
                left,
                current.right(),
            ));
            true
        }
        _ => false,
    }
}
