use super::applied;
use crate::features::layout_engine::domain::{
    AlignItems, FlexDirection, Gap, JustifyContent, PositionType,
};
use crate::features::styling::adapters::lightningcss::properties::flex::{
    apply_flex_container_properties, apply_flex_item_properties,
};
use crate::features::styling::domain::{CssLength, FlexGrow, FlexShrink};

#[test]
fn gap_shorthand_sets_gap_and_row_from_the_row_value_and_column_from_its_own() {
    let (style, handled) = applied(apply_flex_container_properties, "gap: 3px 7px;");
    assert_eq!(handled, [true]);
    assert_eq!(style.gap(), Some(&Gap::new(3.0)));
    assert_eq!(style.row_gap(), Some(&Gap::new(3.0)));
    assert_eq!(style.column_gap(), Some(&Gap::new(7.0)));
}

#[test]
fn unsupported_flex_direction_is_handled_but_not_set() {
    let (style, handled) = applied(
        apply_flex_container_properties,
        "flex-direction: column-reverse;",
    );
    assert_eq!(handled, [true]);
    assert_eq!(style.flex_direction(), None);
    let (style, _) = applied(apply_flex_container_properties, "flex-direction: column;");
    assert_eq!(style.flex_direction(), Some(FlexDirection::Column));
}

#[test]
fn container_alignment_defaults_to_start() {
    let (style, handled) = applied(
        apply_flex_container_properties,
        "justify-content: space-evenly; align-items: baseline;",
    );
    assert_eq!(handled, [true, true]);
    assert_eq!(style.justify_content(), Some(JustifyContent::SpaceEvenly));
    assert_eq!(style.align_items(), Some(AlignItems::Start));
    let (style, _) = applied(
        apply_flex_container_properties,
        "justify-content: normal; align-items: stretch;",
    );
    assert_eq!(style.justify_content(), Some(JustifyContent::Start));
    assert_eq!(style.align_items(), Some(AlignItems::Stretch));
}

#[test]
fn position_is_handled_for_every_keyword() {
    let (style, handled) = applied(apply_flex_container_properties, "position: absolute;");
    assert_eq!(handled, [true]);
    assert_eq!(style.position(), Some(PositionType::Absolute));
    let (style, handled) = applied(apply_flex_container_properties, "position: sticky;");
    assert_eq!(handled, [true]);
    assert_eq!(style.position(), Some(PositionType::Relative));
}

#[test]
fn flex_container_applier_declines_item_and_gap_longhands() {
    let (_, handled) = applied(
        apply_flex_container_properties,
        "flex-grow: 1; column-gap: 2px; align-self: center;",
    );
    assert_eq!(handled, [false, false, false]);
}

#[test]
fn flex_item_longhands_are_each_handled() {
    let (style, handled) = applied(
        apply_flex_item_properties,
        "flex-grow: 4; flex-shrink: 0; flex-basis: auto; align-self: end;",
    );
    assert_eq!(handled, [true; 4]);
    assert_eq!(style.flex_grow(), Some(FlexGrow::new(4.0).unwrap()));
    assert_eq!(style.flex_shrink(), Some(FlexShrink::new(0.0).unwrap()));
    assert_eq!(style.flex_basis(), Some(CssLength::Auto));
    assert_eq!(style.align_self(), Some(AlignItems::End));
}

#[test]
fn flex_shorthand_writes_grow_shrink_and_basis_to_their_own_fields() {
    let (style, handled) = applied(apply_flex_item_properties, "flex: 5 6 17px;");
    assert_eq!(handled, [true]);
    assert_eq!(style.flex_grow(), Some(FlexGrow::new(5.0).unwrap()));
    assert_eq!(style.flex_shrink(), Some(FlexShrink::new(6.0).unwrap()));
    assert_eq!(style.flex_basis(), Some(CssLength::Pixels(17.0)));
}

#[test]
fn align_self_maps_like_align_items() {
    for (keyword, expected) in [
        ("center", AlignItems::Center),
        ("flex-end", AlignItems::End),
        ("stretch", AlignItems::Stretch),
        ("baseline", AlignItems::Start),
    ] {
        let (style, _) = applied(
            apply_flex_item_properties,
            &format!("align-self: {keyword};"),
        );
        assert_eq!(style.align_self(), Some(expected), "{keyword}");
    }
}

#[test]
fn flex_item_applier_declines_container_properties() {
    let (_, handled) = applied(
        apply_flex_item_properties,
        "gap: 1px; align-items: center; order: 2;",
    );
    assert_eq!(handled, [false, false, false]);
}
