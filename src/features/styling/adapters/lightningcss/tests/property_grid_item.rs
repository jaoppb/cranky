use super::declared;
use crate::features::layout_engine::domain::AlignItems;
use crate::features::styling::domain::{GridLinePlacement, GridPlacement};

#[test]
fn justify_items_and_justify_self_map_every_keyword() {
    let cases = [
        ("center", AlignItems::Center),
        ("end", AlignItems::End),
        ("stretch", AlignItems::Stretch),
        ("start", AlignItems::Start),
    ];
    for (keyword, expected) in cases {
        let style = declared(&format!(
            "justify-items: {keyword}; justify-self: {keyword};"
        ));
        assert_eq!(style.justify_items(), Some(expected), "{keyword}");
        assert_eq!(style.justify_self(), Some(expected), "{keyword}");
    }
}

#[test]
fn grid_line_shorthand_reads_start_and_end() {
    let style = declared("grid-column: 2 / span 3; grid-row: -1;");
    assert_eq!(
        style.grid_column(),
        Some(&GridLinePlacement::new(
            GridPlacement::Line(2),
            GridPlacement::Span(3)
        ))
    );
    assert_eq!(
        style.grid_row(),
        Some(&GridLinePlacement::new(
            GridPlacement::Line(-1),
            GridPlacement::Auto
        ))
    );
}

#[test]
fn grid_line_longhand_keeps_the_other_end() {
    let column = declared("grid-column: 2 / 5; grid-column-start: span 2;");
    assert_eq!(
        column.grid_column(),
        Some(&GridLinePlacement::new(
            GridPlacement::Span(2),
            GridPlacement::Line(5)
        ))
    );
    let column = declared("grid-column: 2 / 5; grid-column-end: 7;");
    assert_eq!(
        column.grid_column(),
        Some(&GridLinePlacement::new(
            GridPlacement::Line(2),
            GridPlacement::Line(7)
        ))
    );
    let row = declared("grid-row: 2 / 5; grid-row-start: 3;");
    assert_eq!(
        row.grid_row(),
        Some(&GridLinePlacement::new(
            GridPlacement::Line(3),
            GridPlacement::Line(5)
        ))
    );
    let row = declared("grid-row: 2 / 5; grid-row-end: span 4;");
    assert_eq!(
        row.grid_row(),
        Some(&GridLinePlacement::new(
            GridPlacement::Line(2),
            GridPlacement::Span(4)
        ))
    );
}

#[test]
fn grid_line_longhand_alone_leaves_the_other_end_auto() {
    let style = declared("grid-column-end: 4; grid-row-start: 2;");
    assert_eq!(
        style.grid_column(),
        Some(&GridLinePlacement::new(
            GridPlacement::Auto,
            GridPlacement::Line(4)
        ))
    );
    assert_eq!(
        style.grid_row(),
        Some(&GridLinePlacement::new(
            GridPlacement::Line(2),
            GridPlacement::Auto
        ))
    );
}

#[test]
fn grid_line_auto_keyword_is_an_auto_placement_on_either_side() {
    let style = declared("grid-column: auto / 3; grid-row: span 2 / auto;");
    assert_eq!(
        style.grid_column(),
        Some(&GridLinePlacement::new(
            GridPlacement::Auto,
            GridPlacement::Line(3)
        ))
    );
    assert_eq!(
        style.grid_row(),
        Some(&GridLinePlacement::new(
            GridPlacement::Span(2),
            GridPlacement::Auto
        ))
    );
}
