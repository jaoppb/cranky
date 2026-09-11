use crate::features::layout_engine::domain::{AlignItems, BoxMargin, Gap, JustifyContent};
use crate::features::styling::adapters::lightningcss::LightningCssAdapter;
use crate::features::styling::domain::{
    ClassName, DisplayMode, ElementQuery, GridAutoFlow, GridLinePlacement, GridPlacement,
    GridTrack, InheritedStyle, StyleSheetName,
};
use crate::features::styling::ports::CssParserPort;
use crate::shared::primitives::color::{Color, DrawingColor};

#[test]
fn test_structural_pseudo_classes() {
    let parser = LightningCssAdapter::new();
    let css = r"
        .item:first-child {
            padding-left: 10px;
        }
        .item:last-child {
            padding-right: 15px;
        }
        .item:only-child {
            border-radius: 8px;
        }
        .item:nth-child(2n+1) {
            background-color: #111111;
        }
        .item:nth-child(2n) {
            background-color: #222222;
        }
        .item:empty {
            opacity: 0.5;
        }
        .item:not(.active) {
            color: #888888;
        }
    ";
    let parsed = parser
        .parse_stylesheet(StyleSheetName::new("test").unwrap(), css)
        .unwrap();

    let class_item = ClassName::new("item").unwrap();
    let class_active = ClassName::new("active").unwrap();

    // 1. First child in a list of 3 (index 0, total 3)
    let query_first = ElementQuery::new("flex", None, std::slice::from_ref(&class_item), &[], None)
        .with_structural_context(0, 3, false);
    let style_first = parsed.resolve_style(&query_first, &InheritedStyle::default());
    assert_eq!(style_first.padding().map(BoxMargin::left), Some(10.0));
    assert_eq!(style_first.padding().map(BoxMargin::right), Some(0.0));
    assert!(style_first.background().is_some());
    // Odd: 1st is 2n+1 -> #111111
    if let Some(DrawingColor::Solid(c)) = style_first.background() {
        assert_eq!(*c, Color::new(17, 17, 17, 255));
    }

    // 2. Second child (index 1, total 3)
    let query_second =
        ElementQuery::new("flex", None, std::slice::from_ref(&class_item), &[], None)
            .with_structural_context(1, 3, false);
    let style_second = parsed.resolve_style(&query_second, &InheritedStyle::default());
    assert!(style_second.padding().is_none());
    // Even: 2nd is 2n -> #222222
    if let Some(DrawingColor::Solid(c)) = style_second.background() {
        assert_eq!(*c, Color::new(34, 34, 34, 255));
    }

    // 3. Last child (index 2, total 3)
    let query_last = ElementQuery::new("flex", None, std::slice::from_ref(&class_item), &[], None)
        .with_structural_context(2, 3, false);
    let style_last = parsed.resolve_style(&query_last, &InheritedStyle::default());
    assert_eq!(style_last.padding().map(BoxMargin::right), Some(15.0));

    // 4. Only child (index 0, total 1)
    let query_only = ElementQuery::new("flex", None, std::slice::from_ref(&class_item), &[], None)
        .with_structural_context(0, 1, false);
    let style_only = parsed.resolve_style(&query_only, &InheritedStyle::default());
    assert_eq!(style_only.border_radius().map(|r| r.value()), Some(8.0));

    // 5. Empty
    let query_empty = ElementQuery::new("flex", None, std::slice::from_ref(&class_item), &[], None)
        .with_structural_context(0, 1, true);
    let style_empty = parsed.resolve_style(&query_empty, &InheritedStyle::default());
    assert!((style_empty.opacity().unwrap().value() - 0.5).abs() < f32::EPSILON);

    // 6. Not active vs Active
    let query_not_active =
        ElementQuery::new("flex", None, std::slice::from_ref(&class_item), &[], None);
    let style_not_active = parsed.resolve_style(&query_not_active, &InheritedStyle::default());
    assert!(style_not_active.color().is_some());

    let active_classes = [class_item.clone(), class_active];
    let query_active = ElementQuery::new("flex", None, &active_classes, &[], None);
    let style_active = parsed.resolve_style(&query_active, &InheritedStyle::default());
    assert!(style_active.color().is_none());
}

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
    let style_grid = parsed.resolve_style(&query_grid, &InheritedStyle::default());

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
    let style_item = parsed.resolve_style(&query_item, &InheritedStyle::default());

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
