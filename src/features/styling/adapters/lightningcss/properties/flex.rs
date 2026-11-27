//! Applies flex container properties (direction, alignment, gap) and flex item properties
//! (`flex`, grow, shrink, basis, `align-self`).
use super::box_model::{parse_length_str, parse_size_str};
use crate::features::layout_engine::domain::{
    AlignItems, FlexDirection, Gap, JustifyContent, PositionType,
};
use crate::features::styling::domain::{ComputedStyle, FlexGrow, FlexShrink};
use lightningcss::properties::Property;
use lightningcss::properties::align::Gap as GapShorthand;
use lightningcss::properties::position::Position;
use lightningcss::stylesheet::PrinterOptions;
use lightningcss::traits::ToCss;

pub(crate) fn apply_flex_container_properties(
    style: &mut ComputedStyle,
    prop: &Property<'_>,
) -> bool {
    match prop {
        Property::Gap(gap) => apply_gap(style, gap),
        Property::FlexDirection(dir, _) => apply_flex_direction(style, dir),
        Property::JustifyContent(justify, _) => {
            style.set_justify_content(justify_content_from_css(&css_text(justify)));
        }
        Property::AlignItems(align, _) => {
            style.set_align_items(align_items_from_css(&css_text(align)));
        }
        Property::Position(pos) => style.set_position(position_type(pos)),
        _ => return false,
    }
    true
}

pub(crate) fn apply_flex_item_properties(style: &mut ComputedStyle, prop: &Property<'_>) -> bool {
    match prop {
        Property::FlexGrow(grow, _) => set_flex_grow(style, *grow),
        Property::FlexShrink(shrink, _) => set_flex_shrink(style, *shrink),
        Property::FlexBasis(basis, _) => set_flex_basis(style, basis),
        Property::Flex(flex, _) => {
            set_flex_grow(style, flex.grow);
            set_flex_shrink(style, flex.shrink);
            set_flex_basis(style, &flex.basis);
        }
        Property::AlignSelf(align, _) => {
            style.set_align_self(align_items_from_css(&css_text(align)));
        }
        _ => return false,
    }
    true
}

/// The serialized text of `value`, empty when it does not serialize.
fn css_text(value: &impl ToCss) -> String {
    value
        .to_css_string(PrinterOptions::default())
        .unwrap_or_default()
}

/// Sets `gap` and `row-gap` from the row value and `column-gap` from the column one.
fn apply_gap(style: &mut ComputedStyle, gap: &GapShorthand) {
    let row = parse_length_str(&css_text(&gap.row));
    style.set_gap(Gap::new(f64::from(row)));
    style.set_row_gap(Gap::new(f64::from(row)));
    let col = parse_length_str(&css_text(&gap.column));
    style.set_column_gap(Gap::new(f64::from(col)));
}

/// Sets `flex-direction` when it is `row` or `column`, and leaves it for the reversed ones.
fn apply_flex_direction(style: &mut ComputedStyle, dir: &impl ToCss) {
    let flex_direction = match css_text(dir).as_str() {
        "column" => Some(FlexDirection::Column),
        "row" => Some(FlexDirection::Row),
        _ => None,
    };
    if let Some(direction) = flex_direction {
        style.set_flex_direction(direction);
    }
}

/// The content distribution a serialized `justify-content` or `align-content` names, matched
/// anywhere in the text (`safe center` is centred), `Start` when none is.
pub(super) fn justify_content_from_css(css_text: &str) -> JustifyContent {
    if css_text.contains("space-between") {
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
    }
}

/// The alignment a serialized `align-items` or `align-self` names, matched anywhere in the text,
/// `Start` when none is.
fn align_items_from_css(css_text: &str) -> AlignItems {
    if css_text.contains("center") {
        AlignItems::Center
    } else if css_text.contains("end") || css_text.contains("flex-end") {
        AlignItems::End
    } else if css_text.contains("stretch") {
        AlignItems::Stretch
    } else {
        AlignItems::Start
    }
}

/// `Absolute` for `position: absolute`, `Relative` for every other keyword.
const fn position_type(pos: &Position) -> PositionType {
    match pos {
        Position::Absolute => PositionType::Absolute,
        _ => PositionType::Relative,
    }
}

/// Sets `flex-grow` when the value is a valid factor.
fn set_flex_grow(style: &mut ComputedStyle, grow: f32) {
    if let Ok(val) = FlexGrow::new(grow) {
        style.set_flex_grow(val);
    }
}

/// Sets `flex-shrink` when the value is a valid factor.
fn set_flex_shrink(style: &mut ComputedStyle, shrink: f32) {
    if let Ok(val) = FlexShrink::new(shrink) {
        style.set_flex_shrink(val);
    }
}

/// Sets `flex-basis` when it serializes to a length.
fn set_flex_basis(style: &mut ComputedStyle, basis: &impl ToCss) {
    if let Some(val) = parse_size_str(&css_text(basis)) {
        style.set_flex_basis(val);
    }
}
