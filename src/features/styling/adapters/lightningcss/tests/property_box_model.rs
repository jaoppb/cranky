use super::declared;
use crate::features::layout_engine::domain::BoxMargin;
use crate::features::styling::domain::{ComputedStyle, CssLength};

type Sides = fn(&ComputedStyle) -> Option<&BoxMargin>;

/// `padding` and `margin` share one shape: a four-side shorthand plus a
/// longhand per side.
const BOX_PROPERTIES: [(&str, Sides); 2] = [
    ("padding", ComputedStyle::padding),
    ("margin", ComputedStyle::margin),
];

#[test]
fn box_shorthand_maps_css_order_to_each_side() {
    // CSS order is top, right, bottom, left; `BoxMargin::new` takes top,
    // bottom, left, right.
    for (property, sides) in BOX_PROPERTIES {
        let style = declared(&format!("{property}: 1px 2px 3px 4px;"));
        assert_eq!(
            sides(&style),
            Some(&BoxMargin::new(1.0, 3.0, 4.0, 2.0)),
            "{property}"
        );
    }
}

#[test]
fn box_longhand_replaces_only_its_own_side() {
    let cases = [
        ("top", BoxMargin::new(9.0, 3.0, 4.0, 2.0)),
        ("right", BoxMargin::new(1.0, 3.0, 4.0, 9.0)),
        ("bottom", BoxMargin::new(1.0, 9.0, 4.0, 2.0)),
        ("left", BoxMargin::new(1.0, 3.0, 9.0, 2.0)),
    ];
    for (property, sides) in BOX_PROPERTIES {
        for (side, expected) in &cases {
            let css = format!("{property}: 1px 2px 3px 4px; {property}-{side}: 9px;");
            assert_eq!(sides(&declared(&css)), Some(expected), "{css}");
        }
    }
}

#[test]
fn box_longhand_alone_leaves_the_other_sides_at_zero() {
    for (property, sides) in BOX_PROPERTIES {
        let style = declared(&format!("{property}-left: 7px;"));
        assert_eq!(
            sides(&style),
            Some(&BoxMargin::new(0.0, 0.0, 7.0, 0.0)),
            "{property}"
        );
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
