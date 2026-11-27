use super::applied;
use crate::features::layout_engine::domain::{AlignItems, Gap, JustifyContent};
use crate::features::styling::adapters::lightningcss::properties::grid::{
    apply_display_and_gap_properties, apply_grid_container_properties, apply_grid_item_properties,
};
use crate::features::styling::domain::{
    DisplayMode, GridAutoFlow, GridLinePlacement, GridPlacement, GridTrack,
};

#[test]
fn only_a_supported_display_mode_is_handled() {
    let (style, handled) = applied(
        apply_display_and_gap_properties,
        "display: flex; display: inline-block;",
    );
    assert_eq!(handled, [true, false]);
    assert_eq!(style.display(), Some(DisplayMode::Flex));
}

#[test]
fn gap_longhands_are_handled_into_their_own_axis() {
    let (style, handled) = applied(
        apply_display_and_gap_properties,
        "column-gap: 6px; row-gap: 1em;",
    );
    assert_eq!(handled, [true, true]);
    assert_eq!(style.column_gap(), Some(&Gap::new(6.0)));
    assert_eq!(style.row_gap(), Some(&Gap::new(16.0)));
    assert_eq!(style.gap(), None);
}

#[test]
fn display_and_gap_applier_declines_other_properties() {
    let (_, handled) = applied(
        apply_display_and_gap_properties,
        "gap: 2px; visibility: hidden; grid-row: 1;",
    );
    assert_eq!(handled, [false, false, false]);
}

#[test]
fn each_track_property_is_handled_into_its_own_field() {
    let (style, handled) = applied(
        apply_grid_container_properties,
        "grid-template-columns: 1px; grid-template-rows: 2px; \
         grid-auto-columns: 3px; grid-auto-rows: 4px;",
    );
    assert_eq!(handled, [true; 4]);
    assert_eq!(
        style.grid_template_columns(),
        Some([GridTrack::Pixels(1.0)].as_slice())
    );
    assert_eq!(
        style.grid_template_rows(),
        Some([GridTrack::Pixels(2.0)].as_slice())
    );
    assert_eq!(
        style.grid_auto_columns(),
        Some([GridTrack::Pixels(3.0)].as_slice())
    );
    assert_eq!(
        style.grid_auto_rows(),
        Some([GridTrack::Pixels(4.0)].as_slice())
    );
}

#[test]
fn grid_auto_flow_maps_dense_only_when_spelled_with_its_direction() {
    for (keyword, expected) in [
        ("dense", GridAutoFlow::Row),
        ("dense column", GridAutoFlow::ColumnDense),
        ("column dense", GridAutoFlow::ColumnDense),
        ("column", GridAutoFlow::Column),
    ] {
        let (style, handled) = applied(
            apply_grid_container_properties,
            &format!("grid-auto-flow: {keyword};"),
        );
        assert_eq!(handled, [true], "{keyword}");
        assert_eq!(style.grid_auto_flow(), Some(expected), "{keyword}");
    }
}

#[test]
fn justify_items_falls_back_to_start() {
    for (keyword, expected) in [
        ("flex-end", AlignItems::End),
        ("center", AlignItems::Center),
        ("stretch", AlignItems::Stretch),
        ("baseline", AlignItems::Start),
    ] {
        let (style, handled) = applied(
            apply_grid_container_properties,
            &format!("justify-items: {keyword};"),
        );
        assert_eq!(handled, [true], "{keyword}");
        assert_eq!(style.justify_items(), Some(expected), "{keyword}");
    }
}

#[test]
fn align_content_matches_keywords_inside_the_value() {
    for (keyword, expected) in [
        ("flex-end", JustifyContent::End),
        ("safe center", JustifyContent::Center),
        ("normal", JustifyContent::Start),
        ("space-around", JustifyContent::SpaceAround),
    ] {
        let (style, handled) = applied(
            apply_grid_container_properties,
            &format!("align-content: {keyword};"),
        );
        assert_eq!(handled, [true], "{keyword}");
        assert_eq!(style.align_content(), Some(expected), "{keyword}");
    }
}

#[test]
fn grid_container_applier_declines_other_properties() {
    let (_, handled) = applied(
        apply_grid_container_properties,
        "display: grid; grid-column: 1; justify-self: end; align-items: end;",
    );
    assert_eq!(handled, [false; 4]);
}

#[test]
fn every_grid_item_property_is_handled() {
    let (style, handled) = applied(
        apply_grid_item_properties,
        "grid-column: 1 / 2; grid-column-start: 3; grid-column-end: 4; \
         grid-row: 5 / 6; grid-row-start: span 7; grid-row-end: 8; justify-self: stretch;",
    );
    assert_eq!(handled, [true; 7]);
    assert_eq!(
        style.grid_column(),
        Some(&GridLinePlacement::new(
            GridPlacement::Line(3),
            GridPlacement::Line(4)
        ))
    );
    assert_eq!(
        style.grid_row(),
        Some(&GridLinePlacement::new(
            GridPlacement::Span(7),
            GridPlacement::Line(8)
        ))
    );
    assert_eq!(style.justify_self(), Some(AlignItems::Stretch));
}

#[test]
fn grid_line_end_alone_keeps_an_auto_start() {
    let (style, _) = applied(apply_grid_item_properties, "grid-row-end: span 2;");
    assert_eq!(
        style.grid_row(),
        Some(&GridLinePlacement::new(
            GridPlacement::Auto,
            GridPlacement::Span(2)
        ))
    );
    let (style, _) = applied(apply_grid_item_properties, "grid-column-start: 2;");
    assert_eq!(
        style.grid_column(),
        Some(&GridLinePlacement::new(
            GridPlacement::Line(2),
            GridPlacement::Auto
        ))
    );
    assert_eq!(style.grid_row(), None);
}

#[test]
fn justify_self_falls_back_to_start() {
    for (keyword, expected) in [
        ("flex-end", AlignItems::End),
        ("center", AlignItems::Center),
        ("baseline", AlignItems::Start),
    ] {
        let (style, _) = applied(
            apply_grid_item_properties,
            &format!("justify-self: {keyword};"),
        );
        assert_eq!(style.justify_self(), Some(expected), "{keyword}");
    }
}

#[test]
fn grid_item_applier_declines_other_properties() {
    let (_, handled) = applied(
        apply_grid_item_properties,
        "grid-template-rows: 1px; justify-items: end; grid-area: 1;",
    );
    assert_eq!(handled, [false; 3]);
}
