use super::box_model::{parse_length_str, parse_size_str};
use crate::features::layout_engine::domain::{
    AlignItems, FlexDirection, Gap, JustifyContent, PositionType,
};
use crate::features::styling::domain::{ComputedStyle, FlexGrow, FlexShrink};
use lightningcss::properties::Property;
use lightningcss::stylesheet::PrinterOptions;
use lightningcss::traits::ToCss;

pub(crate) fn apply_flex_container_properties(
    style: &mut ComputedStyle,
    prop: &Property<'_>,
) -> bool {
    match prop {
        Property::Gap(gap) => {
            let row_str = gap
                .row
                .to_css_string(PrinterOptions::default())
                .unwrap_or_default();
            let row = parse_length_str(&row_str);
            style.set_gap(Gap::new(f64::from(row)));
            style.set_row_gap(Gap::new(f64::from(row)));
            let col_str = gap
                .column
                .to_css_string(PrinterOptions::default())
                .unwrap_or_default();
            let col = parse_length_str(&col_str);
            style.set_column_gap(Gap::new(f64::from(col)));
            true
        }
        Property::FlexDirection(dir, _) => {
            let css_text = dir
                .to_css_string(PrinterOptions::default())
                .unwrap_or_default();
            let flex_direction = match css_text.as_str() {
                "column" => Some(FlexDirection::Column),
                "row" => Some(FlexDirection::Row),
                _ => None,
            };
            if let Some(direction) = flex_direction {
                style.set_flex_direction(direction);
            }
            true
        }
        Property::JustifyContent(justify, _) => {
            let css_text = justify
                .to_css_string(PrinterOptions::default())
                .unwrap_or_default();
            let val = if css_text.contains("space-between") {
                JustifyContent::SpaceBetween
            } else if css_text.contains("space-around") {
                JustifyContent::SpaceAround
            } else if css_text.contains("space-evenly") {
                JustifyContent::SpaceEvenly
            } else if css_text.contains("center") {
                JustifyContent::Center
            } else if css_text.contains("end") || css_text.contains("flex-end") {
                JustifyContent::End
            } else {
                JustifyContent::Start
            };
            style.set_justify_content(val);
            true
        }
        Property::AlignItems(align, _) => {
            let css_text = align
                .to_css_string(PrinterOptions::default())
                .unwrap_or_default();
            let val = if css_text.contains("center") {
                AlignItems::Center
            } else if css_text.contains("end") || css_text.contains("flex-end") {
                AlignItems::End
            } else if css_text.contains("stretch") {
                AlignItems::Stretch
            } else {
                AlignItems::Start
            };
            style.set_align_items(val);
            true
        }
        Property::Position(pos) => {
            let val = match pos {
                lightningcss::properties::position::Position::Absolute => PositionType::Absolute,
                _ => PositionType::Relative,
            };
            style.set_position(val);
            true
        }
        _ => false,
    }
}

pub(crate) fn apply_flex_item_properties(style: &mut ComputedStyle, prop: &Property<'_>) -> bool {
    match prop {
        Property::FlexGrow(grow, _) => {
            if let Ok(val) = FlexGrow::new(*grow) {
                style.set_flex_grow(val);
            }
            true
        }
        Property::FlexShrink(shrink, _) => {
            if let Ok(val) = FlexShrink::new(*shrink) {
                style.set_flex_shrink(val);
            }
            true
        }
        Property::FlexBasis(basis, _) => {
            let css_text = basis
                .to_css_string(PrinterOptions::default())
                .unwrap_or_default();
            if let Some(val) = parse_size_str(&css_text) {
                style.set_flex_basis(val);
            }
            true
        }
        Property::Flex(flex, _) => {
            if let Ok(val) = FlexGrow::new(flex.grow) {
                style.set_flex_grow(val);
            }
            if let Ok(val) = FlexShrink::new(flex.shrink) {
                style.set_flex_shrink(val);
            }
            let css_text = flex
                .basis
                .to_css_string(PrinterOptions::default())
                .unwrap_or_default();
            if let Some(val) = parse_size_str(&css_text) {
                style.set_flex_basis(val);
            }
            true
        }
        Property::AlignSelf(as_, _) => {
            let css_text = as_
                .to_css_string(PrinterOptions::default())
                .unwrap_or_default();
            let val = if css_text.contains("center") {
                AlignItems::Center
            } else if css_text.contains("end") || css_text.contains("flex-end") {
                AlignItems::End
            } else if css_text.contains("stretch") {
                AlignItems::Stretch
            } else {
                AlignItems::Start
            };
            style.set_align_self(val);
            true
        }
        _ => false,
    }
}
