use crate::features::layout_engine::domain::BoxMargin;
use crate::features::styling::adapters::lightningcss::{
    LightningCssAdapter, LightningPropertyReparser,
};
use crate::features::styling::domain::{ClassName, ElementQuery, InheritedStyle, StyleSheetName};
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
    let (style_first, _) = parsed.resolve_style(
        &query_first,
        &InheritedStyle::default(),
        &LightningPropertyReparser,
    );
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
    let (style_second, _) = parsed.resolve_style(
        &query_second,
        &InheritedStyle::default(),
        &LightningPropertyReparser,
    );
    assert!(style_second.padding().is_none());
    // Even: 2nd is 2n -> #222222
    if let Some(DrawingColor::Solid(c)) = style_second.background() {
        assert_eq!(*c, Color::new(34, 34, 34, 255));
    }

    // 3. Last child (index 2, total 3)
    let query_last = ElementQuery::new("flex", None, std::slice::from_ref(&class_item), &[], None)
        .with_structural_context(2, 3, false);
    let (style_last, _) = parsed.resolve_style(
        &query_last,
        &InheritedStyle::default(),
        &LightningPropertyReparser,
    );
    assert_eq!(style_last.padding().map(BoxMargin::right), Some(15.0));

    // 4. Only child (index 0, total 1)
    let query_only = ElementQuery::new("flex", None, std::slice::from_ref(&class_item), &[], None)
        .with_structural_context(0, 1, false);
    let (style_only, _) = parsed.resolve_style(
        &query_only,
        &InheritedStyle::default(),
        &LightningPropertyReparser,
    );
    assert_eq!(style_only.border_radius().map(|r| r.value()), Some(8.0));

    // 5. Empty
    let query_empty = ElementQuery::new("flex", None, std::slice::from_ref(&class_item), &[], None)
        .with_structural_context(0, 1, true);
    let (style_empty, _) = parsed.resolve_style(
        &query_empty,
        &InheritedStyle::default(),
        &LightningPropertyReparser,
    );
    assert!((style_empty.opacity().unwrap().value() - 0.5).abs() < f32::EPSILON);

    // 6. Not active vs Active
    let query_not_active =
        ElementQuery::new("flex", None, std::slice::from_ref(&class_item), &[], None);
    let (style_not_active, _) = parsed.resolve_style(
        &query_not_active,
        &InheritedStyle::default(),
        &LightningPropertyReparser,
    );
    assert!(style_not_active.color().is_some());

    let active_classes = [class_item.clone(), class_active];
    let query_active = ElementQuery::new("flex", None, &active_classes, &[], None);
    let (style_active, _) = parsed.resolve_style(
        &query_active,
        &InheritedStyle::default(),
        &LightningPropertyReparser,
    );
    assert!(style_active.color().is_none());
}
