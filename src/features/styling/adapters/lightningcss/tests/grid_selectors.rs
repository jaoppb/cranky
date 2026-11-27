use super::support::{class_style, parse};
use crate::features::layout_engine::domain::{AlignItems, Gap, JustifyContent};
use crate::features::styling::domain::{
    DisplayMode, GridAutoFlow, GridLinePlacement, GridPlacement, GridTrack,
};

/// The sheet the grid tests read: a grid container with every grid property
/// it sets and an item placed in it.
const GRID_CSS: &str = r"
        grid.container {
            display: grid;
            grid-template-columns: repeat(3, 1fr);
            grid-template-rows: 50px auto minmax(20px, 1fr);
            grid-auto-flow: column dense;
            column-gap: 12px;
            row-gap: 8px;
            justify-items: center;
            align-content: space-between;
        }
        .item {
            grid-column: 1 / span 2;
            grid-row: span 3;
            justify-self: end;
        }
    ";

/// The row tracks `grid-template-rows: 50px auto minmax(20px, 1fr)` reads as.
fn expected_rows() -> Vec<GridTrack> {
    vec![
        GridTrack::Pixels(50.0),
        GridTrack::Auto,
        GridTrack::MinMax(
            Box::new(GridTrack::Pixels(20.0)),
            Box::new(GridTrack::Fraction(1.0)),
        ),
    ]
}

#[test]
fn test_grid_properties_parsing() {
    let parsed = parse("test_grid", GRID_CSS);
    let style_grid = class_style(&*parsed, "grid", "container");

    assert_eq!(style_grid.display(), Some(DisplayMode::Grid));
    assert_eq!(
        style_grid.grid_template_columns(),
        Some(&[GridTrack::Repeat(3, vec![GridTrack::Fraction(1.0)])][..])
    );
    assert_eq!(style_grid.grid_template_rows(), Some(&expected_rows()[..]));
    assert_eq!(style_grid.grid_auto_flow(), Some(GridAutoFlow::ColumnDense));
    let gaps = (
        style_grid.column_gap().map(Gap::value),
        style_grid.row_gap().map(Gap::value),
    );
    assert_eq!(gaps, (Some(12.0), Some(8.0)));
    assert_eq!(style_grid.justify_items(), Some(AlignItems::Center));
    assert_eq!(
        style_grid.align_content(),
        Some(JustifyContent::SpaceBetween)
    );
}

/// The `.item` half of the same sheet: its line placements and its own justification.
#[test]
fn test_grid_item_properties_parsing() {
    let parsed = parse("test_grid", GRID_CSS);
    let style_item = class_style(&*parsed, "flex", "item");

    assert_eq!(
        style_item.grid_column(),
        Some(&GridLinePlacement::new(
            GridPlacement::Line(1),
            GridPlacement::Span(2)
        ))
    );
    assert_eq!(
        style_item.grid_row(),
        Some(&GridLinePlacement::new(
            GridPlacement::Span(3),
            GridPlacement::Auto
        ))
    );
    assert_eq!(style_item.justify_self(), Some(AlignItems::End));
}
