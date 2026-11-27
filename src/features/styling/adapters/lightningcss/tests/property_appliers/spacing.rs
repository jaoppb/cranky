use super::applied;
use crate::features::layout_engine::domain::BoxMargin;
use crate::features::styling::adapters::lightningcss::properties::box_model::{
    apply_margin_properties, apply_padding_properties,
};
use crate::features::styling::adapters::lightningcss::properties::detect_relative_lengths;
use crate::features::styling::domain::ComputedStyle;
use crate::features::styling::domain::cascade::declared_lengths_apply::FontSizeBases;
use crate::shared::config::domain::{AxisMargins, FontSize};
use lightningcss::rules::CssRule;
use lightningcss::stylesheet::{ParserOptions, StyleSheet};

/// `BoxMargin` from the CSS side order: top, right, bottom, left.
fn sides([top, right, bottom, left]: [f64; 4]) -> BoxMargin {
    BoxMargin::new(AxisMargins::new(top, bottom), AxisMargins::new(left, right))
}

#[test]
fn spacing_shorthand_and_each_longhand_write_their_sides() {
    let declarations = |property: &str| {
        format!(
            "{property}: 1px 2px 3px 4px; {property}-top: 5px; {property}-right: 6px; \
             {property}-bottom: 7px; {property}-left: 8px;"
        )
    };
    let (padding, handled) = applied(apply_padding_properties, &declarations("padding"));
    assert_eq!(handled, [true; 5]);
    assert_eq!(padding.padding(), Some(&sides([5.0, 6.0, 7.0, 8.0])));
    assert_eq!(padding.margin(), None);

    let (margin, handled) = applied(apply_margin_properties, &declarations("margin"));
    assert_eq!(handled, [true; 5]);
    assert_eq!(margin.margin(), Some(&sides([5.0, 6.0, 7.0, 8.0])));
    assert_eq!(margin.padding(), None);
}

#[test]
fn spacing_longhand_keeps_the_sides_already_set() {
    for (side, expected) in [
        ("top", sides([9.0, 2.0, 3.0, 4.0])),
        ("right", sides([1.0, 9.0, 3.0, 4.0])),
        ("bottom", sides([1.0, 2.0, 9.0, 4.0])),
        ("left", sides([1.0, 2.0, 3.0, 9.0])),
    ] {
        let css = format!("padding: 1px 2px 3px 4px; padding-{side}: 9px;");
        let (style, _) = applied(apply_padding_properties, &css);
        assert_eq!(style.padding(), Some(&expected), "{css}");
        let css = format!("margin: 1px 2px 3px 4px; margin-{side}: 9px;");
        let (style, _) = applied(apply_margin_properties, &css);
        assert_eq!(style.margin(), Some(&expected), "{css}");
    }
}

#[test]
fn spacing_appliers_decline_each_other_and_the_rest() {
    let (_, handled) = applied(
        apply_padding_properties,
        "margin: 1px; margin-top: 2px; gap: 1px;",
    );
    assert_eq!(handled, [false, false, false]);
    let (_, handled) = applied(
        apply_margin_properties,
        "padding: 1px; padding-left: 2px; gap: 1px;",
    );
    assert_eq!(handled, [false, false, false]);
}

/// The computed style `detect_relative_lengths` leaves for `declarations`,
/// with a 10px element font size and a 100px root one.
fn relative(declarations: &str) -> ComputedStyle {
    let css = format!("flex {{ {declarations} }}");
    let sheet = StyleSheet::parse(&css, ParserOptions::default()).unwrap();
    let CssRule::Style(style_rule) = sheet.rules.0.first().unwrap() else {
        panic!("expected a style rule");
    };
    let lengths = detect_relative_lengths(&style_rule.declarations.declarations);
    let mut computed = ComputedStyle::default();
    lengths.apply(
        &mut computed,
        FontSizeBases::new(FontSize::new(10.0), FontSize::new(100.0)),
    );
    computed
}

#[test]
fn relative_spacing_shorthand_resolves_each_side() {
    let style = relative("padding: 1em 2em 3em 4rem; margin: 1rem 2rem 3em 4em;");
    assert_eq!(style.padding(), Some(&sides([10.0, 20.0, 30.0, 400.0])));
    assert_eq!(style.margin(), Some(&sides([100.0, 200.0, 30.0, 40.0])));
}

#[test]
fn relative_spacing_longhands_resolve_their_own_side() {
    let style = relative(
        "padding-top: 1em; padding-right: 2em; padding-bottom: 3em; padding-left: 4em; \
         margin-top: 5em; margin-right: 6em; margin-bottom: 7em; margin-left: 8em;",
    );
    assert_eq!(style.padding(), Some(&sides([10.0, 20.0, 30.0, 40.0])));
    assert_eq!(style.margin(), Some(&sides([50.0, 60.0, 70.0, 80.0])));
}

#[test]
fn relative_spacing_ignores_pixel_sides() {
    let style = relative("padding-top: 3px; margin: 1px 2em 3px 4px;");
    assert_eq!(style.padding(), None);
    assert_eq!(style.margin(), Some(&sides([0.0, 20.0, 0.0, 0.0])));
}
