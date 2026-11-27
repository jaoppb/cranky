//! Applies `padding` and `margin`, shorthand and per side.
use super::utils::length_to_f64;
use crate::features::layout_engine::domain::BoxMargin;
use crate::features::styling::domain::ComputedStyle;
use crate::shared::config::domain::AxisMargins;
use lightningcss::properties::Property;
use lightningcss::values::length::LengthPercentageOrAuto;

pub(crate) fn apply_padding_properties(style: &mut ComputedStyle, prop: &Property<'_>) -> bool {
    let padding = match prop {
        Property::Padding(padding) => {
            box_from_sides([&padding.top, &padding.right, &padding.bottom, &padding.left])
        }
        Property::PaddingTop(len) => with_side(style.padding(), Side::Top, len),
        Property::PaddingRight(len) => with_side(style.padding(), Side::Right, len),
        Property::PaddingBottom(len) => with_side(style.padding(), Side::Bottom, len),
        Property::PaddingLeft(len) => with_side(style.padding(), Side::Left, len),
        _ => return false,
    };
    style.set_padding(padding);
    true
}

pub(crate) fn apply_margin_properties(style: &mut ComputedStyle, prop: &Property<'_>) -> bool {
    let margin = match prop {
        Property::Margin(margin) => {
            box_from_sides([&margin.top, &margin.right, &margin.bottom, &margin.left])
        }
        Property::MarginTop(len) => with_side(style.margin(), Side::Top, len),
        Property::MarginRight(len) => with_side(style.margin(), Side::Right, len),
        Property::MarginBottom(len) => with_side(style.margin(), Side::Bottom, len),
        Property::MarginLeft(len) => with_side(style.margin(), Side::Left, len),
        _ => return false,
    };
    style.set_margin(margin);
    true
}

/// The side of a box a longhand such as `padding-top` writes.
#[derive(Debug, Clone, Copy)]
enum Side {
    /// `*-top`.
    Top,
    /// `*-right`.
    Right,
    /// `*-bottom`.
    Bottom,
    /// `*-left`.
    Left,
}

/// The box a four-side shorthand declares, its sides in CSS order: top, right, bottom, left.
fn box_from_sides([top, right, bottom, left]: [&LengthPercentageOrAuto; 4]) -> BoxMargin {
    BoxMargin::new(
        AxisMargins::new(length_to_f64(top), length_to_f64(bottom)),
        AxisMargins::new(length_to_f64(left), length_to_f64(right)),
    )
}

/// `current` (zero on every side when undeclared) with only `side` replaced by `length`.
fn with_side(
    current: Option<&BoxMargin>,
    side: Side,
    length: &LengthPercentageOrAuto,
) -> BoxMargin {
    let value = length_to_f64(length);
    let current = current.cloned().unwrap_or_default();
    let (mut top, mut right) = (current.top(), current.right());
    let (mut bottom, mut left) = (current.bottom(), current.left());
    match side {
        Side::Top => top = value,
        Side::Right => right = value,
        Side::Bottom => bottom = value,
        Side::Left => left = value,
    }
    BoxMargin::new(AxisMargins::new(top, bottom), AxisMargins::new(left, right))
}
