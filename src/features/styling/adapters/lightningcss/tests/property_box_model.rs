use super::declared;
use crate::features::layout_engine::domain::BoxMargin;
use crate::features::styling::domain::CssLength;

#[test]
fn padding_shorthand_maps_css_order_to_each_side() {
    // CSS order is top, right, bottom, left; `BoxMargin::new` takes top,
    // bottom, left, right.
    let style = declared("padding: 1px 2px 3px 4px;");
    assert_eq!(style.padding(), Some(&BoxMargin::new(1.0, 3.0, 4.0, 2.0)));
}

#[test]
fn padding_longhand_replaces_only_its_own_side() {
    let cases = [
        ("padding-top: 9px;", BoxMargin::new(9.0, 3.0, 4.0, 2.0)),
        ("padding-right: 9px;", BoxMargin::new(1.0, 3.0, 4.0, 9.0)),
        ("padding-bottom: 9px;", BoxMargin::new(1.0, 9.0, 4.0, 2.0)),
        ("padding-left: 9px;", BoxMargin::new(1.0, 3.0, 9.0, 2.0)),
    ];
    for (longhand, expected) in cases {
        let style = declared(&format!("padding: 1px 2px 3px 4px; {longhand}"));
        assert_eq!(style.padding(), Some(&expected), "{longhand}");
    }
}

#[test]
fn padding_longhand_alone_leaves_the_other_sides_at_zero() {
    let style = declared("padding-left: 7px;");
    assert_eq!(style.padding(), Some(&BoxMargin::new(0.0, 0.0, 7.0, 0.0)));
}

#[test]
fn margin_shorthand_maps_css_order_to_each_side() {
    let style = declared("margin: 1px 2px 3px 4px;");
    assert_eq!(style.margin(), Some(&BoxMargin::new(1.0, 3.0, 4.0, 2.0)));
}

#[test]
fn margin_longhand_replaces_only_its_own_side() {
    let cases = [
        ("margin-top: 9px;", BoxMargin::new(9.0, 3.0, 4.0, 2.0)),
        ("margin-right: 9px;", BoxMargin::new(1.0, 3.0, 4.0, 9.0)),
        ("margin-bottom: 9px;", BoxMargin::new(1.0, 9.0, 4.0, 2.0)),
        ("margin-left: 9px;", BoxMargin::new(1.0, 3.0, 9.0, 2.0)),
    ];
    for (longhand, expected) in cases {
        let style = declared(&format!("margin: 1px 2px 3px 4px; {longhand}"));
        assert_eq!(style.margin(), Some(&expected), "{longhand}");
    }
}

#[test]
fn padding_and_margin_do_not_write_into_each_other() {
    let style = declared("padding-top: 5px; margin-bottom: 6px;");
    assert_eq!(style.padding(), Some(&BoxMargin::new(5.0, 0.0, 0.0, 0.0)));
    assert_eq!(style.margin(), Some(&BoxMargin::new(0.0, 6.0, 0.0, 0.0)));
}

#[test]
fn each_sizing_property_sets_its_own_field() {
    let style = declared(
        "width: 10px; height: 20px; min-width: 30px; max-width: 40px; \
         min-height: 50px; max-height: 60px;",
    );
    assert_eq!(style.width(), Some(CssLength::Px(10.0)));
    assert_eq!(style.height(), Some(CssLength::Px(20.0)));
    assert_eq!(style.min_width(), Some(CssLength::Px(30.0)));
    assert_eq!(style.max_width(), Some(CssLength::Px(40.0)));
    assert_eq!(style.min_height(), Some(CssLength::Px(50.0)));
    assert_eq!(style.max_height(), Some(CssLength::Px(60.0)));
}

#[test]
fn sizing_keeps_percent_and_auto_distinct_from_pixels() {
    let style = declared("width: 50%; height: auto;");
    assert_eq!(style.width(), Some(CssLength::Percent(50.0)));
    assert_eq!(style.height(), Some(CssLength::Auto));
}

#[test]
fn max_size_none_leaves_the_limit_undeclared() {
    let style = declared("max-width: none; max-height: none;");
    assert_eq!(style.max_width(), None);
    assert_eq!(style.max_height(), None);
}
