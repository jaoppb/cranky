use crate::features::layout_engine::domain::{AlignItems, Gap, JustifyContent};
use crate::features::styling::adapters::lightningcss::{
    LightningCssAdapter, LightningPropertyReparser,
};
use crate::features::styling::domain::{
    ClassName, DisplayMode, ElementQuery, GridAutoFlow, GridLinePlacement, GridPlacement,
    GridTrack, InheritedStyle, StyleSheetName,
};
use crate::features::styling::ports::CssParserPort;

#[test]
fn test_grid_properties_parsing() {
    let parser = LightningCssAdapter::new();
    let css = r"
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
    let parsed = parser
        .parse_stylesheet(StyleSheetName::new("test_grid").unwrap(), css)
        .unwrap();

    let class_container = ClassName::new("container").unwrap();
    let query_grid = ElementQuery::new(
        "grid",
        None,
        std::slice::from_ref(&class_container),
        &[],
        None,
    );
    let (style_grid, _) = parsed.resolve_style(
        &query_grid,
        &InheritedStyle::default(),
        &LightningPropertyReparser,
    );

    assert_eq!(style_grid.display(), Some(DisplayMode::Grid));
    assert_eq!(
        style_grid.grid_template_columns(),
        Some(&[GridTrack::Repeat(3, vec![GridTrack::Fr(1.0)])][..])
    );
    assert_eq!(
        style_grid.grid_template_rows(),
        Some(
            &[
                GridTrack::Px(50.0),
                GridTrack::Auto,
                GridTrack::MinMax(Box::new(GridTrack::Px(20.0)), Box::new(GridTrack::Fr(1.0))),
            ][..]
        )
    );
    assert_eq!(style_grid.grid_auto_flow(), Some(GridAutoFlow::ColumnDense));
    assert_eq!(style_grid.column_gap().map(Gap::value), Some(12.0));
    assert_eq!(style_grid.row_gap().map(Gap::value), Some(8.0));
    assert_eq!(style_grid.justify_items(), Some(AlignItems::Center));
    assert_eq!(
        style_grid.align_content(),
        Some(JustifyContent::SpaceBetween)
    );

    let class_item = ClassName::new("item").unwrap();
    let query_item = ElementQuery::new("flex", None, std::slice::from_ref(&class_item), &[], None);
    let (style_item, _) = parsed.resolve_style(
        &query_item,
        &InheritedStyle::default(),
        &LightningPropertyReparser,
    );

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
