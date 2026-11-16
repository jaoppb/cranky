use super::declared;
use crate::features::layout_engine::domain::{
    AlignItems, FlexDirection, Gap, JustifyContent, PositionType,
};
use crate::features::styling::domain::{CssLength, FlexGrow, FlexShrink};

#[test]
fn gap_shorthand_splits_into_row_and_column() {
    let style = declared("gap: 4px 8px;");
    assert_eq!(style.row_gap(), Some(&Gap::new(4.0)));
    assert_eq!(style.column_gap(), Some(&Gap::new(8.0)));
    // The single-axis `gap` a flex container reads follows the row value.
    assert_eq!(style.gap(), Some(&Gap::new(4.0)));
}

#[test]
fn flex_direction_accepts_row_and_column_only() {
    assert_eq!(
        declared("flex-direction: column;").flex_direction(),
        Some(FlexDirection::Column)
    );
    assert_eq!(
        declared("flex-direction: row;").flex_direction(),
        Some(FlexDirection::Row)
    );
    assert_eq!(
        declared("flex-direction: row-reverse;").flex_direction(),
        None
    );
}

#[test]
fn justify_content_maps_every_keyword() {
    let cases = [
        ("space-between", JustifyContent::SpaceBetween),
        ("space-around", JustifyContent::SpaceAround),
        ("space-evenly", JustifyContent::SpaceEvenly),
        ("center", JustifyContent::Center),
        ("flex-end", JustifyContent::End),
        ("end", JustifyContent::End),
        ("flex-start", JustifyContent::Start),
    ];
    for (keyword, expected) in cases {
        let style = declared(&format!("justify-content: {keyword};"));
        assert_eq!(style.justify_content(), Some(expected), "{keyword}");
    }
}

#[test]
fn align_items_maps_every_keyword() {
    let cases = [
        ("center", AlignItems::Center),
        ("flex-end", AlignItems::End),
        ("end", AlignItems::End),
        ("stretch", AlignItems::Stretch),
        ("flex-start", AlignItems::Start),
    ];
    for (keyword, expected) in cases {
        let style = declared(&format!("align-items: {keyword};"));
        assert_eq!(style.align_items(), Some(expected), "{keyword}");
    }
}

#[test]
fn align_self_maps_every_keyword() {
    let cases = [
        ("center", AlignItems::Center),
        ("flex-end", AlignItems::End),
        ("stretch", AlignItems::Stretch),
        ("flex-start", AlignItems::Start),
    ];
    for (keyword, expected) in cases {
        let style = declared(&format!("align-self: {keyword};"));
        assert_eq!(style.align_self(), Some(expected), "{keyword}");
    }
}

#[test]
fn only_absolute_takes_an_element_out_of_flow() {
    assert_eq!(
        declared("position: absolute;").position(),
        Some(PositionType::Absolute)
    );
    assert_eq!(
        declared("position: relative;").position(),
        Some(PositionType::Relative)
    );
    assert_eq!(
        declared("position: fixed;").position(),
        Some(PositionType::Relative)
    );
}

#[test]
fn flex_item_longhands_set_grow_shrink_and_basis() {
    let style = declared("flex-grow: 2; flex-shrink: 3; flex-basis: 40px;");
    assert_eq!(style.flex_grow(), Some(FlexGrow::new(2.0).unwrap()));
    assert_eq!(style.flex_shrink(), Some(FlexShrink::new(3.0).unwrap()));
    assert_eq!(style.flex_basis(), Some(CssLength::Px(40.0)));
}

#[test]
fn flex_shorthand_sets_grow_shrink_and_basis_in_that_order() {
    let style = declared("flex: 2 3 25%;");
    assert_eq!(style.flex_grow(), Some(FlexGrow::new(2.0).unwrap()));
    assert_eq!(style.flex_shrink(), Some(FlexShrink::new(3.0).unwrap()));
    assert_eq!(style.flex_basis(), Some(CssLength::Percent(25.0)));
}
