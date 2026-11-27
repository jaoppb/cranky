//! Pins how `parse_stylesheet` turns each style rule into normal and `!important` entries.
use crate::features::styling::adapters::lightningcss::LightningCssAdapter;
use crate::features::styling::domain::{
    ClassName, ElementIdentity, ElementQuery, StyleSheetName, StylingError,
};
use crate::features::styling::ports::CssParserPort;

/// The debug form of every rule of `css` that matches a `flex` element with class `item`.
fn matches(css: &str) -> Vec<String> {
    let parsed = LightningCssAdapter::new()
        .parse_stylesheet(StyleSheetName::new("test").unwrap(), css)
        .unwrap();
    let classes = [ClassName::new("item").unwrap()];
    let query = ElementQuery::new(ElementIdentity::new("flex", None, &classes), &[], None);
    parsed
        .matching_rules(&query)
        .iter()
        .map(|rule_match| format!("{rule_match:?}"))
        .collect()
}

/// The only match of `importance` (`Normal` or `Important`) among `matches`.
fn entry<'a>(matches: &'a [String], importance: &str) -> &'a str {
    let needle = format!("importance: {importance},");
    let found: Vec<&String> = matches
        .iter()
        .filter(|rule_match| rule_match.contains(&needle))
        .collect();
    assert_eq!(found.len(), 1, "{importance} entries in {matches:#?}");
    found[0]
}

#[test]
fn important_half_is_entered_before_the_normal_half() {
    let matches = matches("flex { padding: 1px; margin: 2px !important; }");
    assert_eq!(matches.len(), 2);
    let important = entry(&matches, "Important");
    assert!(important.contains("source_order: 0 }"), "{important}");
    assert!(important.contains("padding: None, margin: Some(BoxMargin { top: 2.0"));
    let normal = entry(&matches, "Normal");
    assert!(normal.contains("source_order: 1 }"), "{normal}");
    assert!(
        normal.contains("padding: Some(BoxMargin { top: 1.0"),
        "{normal}"
    );
    assert!(normal.contains("margin: None"), "{normal}");
}

#[test]
fn a_half_holding_only_a_css_wide_keyword_is_kept() {
    let matches = matches("flex { color: inherit; font-size: initial !important; }");
    let important = entry(&matches, "Important");
    assert!(important.contains("color: NotDeclared"), "{important}");
    assert!(important.contains("font_size: Initial"), "{important}");
    let normal = entry(&matches, "Normal");
    assert!(normal.contains("color: Inherit"), "{normal}");
    assert!(normal.contains("font_size: NotDeclared"), "{normal}");
}

#[test]
fn a_half_holding_only_custom_properties_is_kept() {
    let matches = matches("flex { --a: 1; --b: 2 !important; }");
    let important = entry(&matches, "Important");
    assert!(important.contains(r#"custom_properties: {"--b": "2"}"#));
    let normal = entry(&matches, "Normal");
    assert!(normal.contains(r#"custom_properties: {"--a": "1"}"#));
}

#[test]
fn a_half_holding_only_var_uses_is_kept() {
    let matches = matches("flex { padding: var(--a); margin: var(--b) !important; }");
    let important = entry(&matches, "Important");
    assert!(important.contains(r#"pending_vars: {"margin": "var(--b)"}"#));
    let normal = entry(&matches, "Normal");
    assert!(normal.contains(r#"pending_vars: {"padding": "var(--a)"}"#));
}

#[test]
fn font_relative_lengths_stay_with_their_own_half() {
    let matches = matches("flex { font-size: 2em; padding: 3em !important; }");
    let important = entry(&matches, "Important");
    assert!(important.contains("font_size: NotDeclared"), "{important}");
    assert!(important.contains("padding_top: Value(Ems(3.0))"));
    let normal = entry(&matches, "Normal");
    assert!(normal.contains("font_size: Value(Ems(2.0))"), "{normal}");
    assert!(normal.contains("padding_top: NotDeclared"), "{normal}");
}

#[test]
fn a_rule_with_nothing_declared_yields_no_entry() {
    assert_eq!(matches("flex { }"), Vec::<String>::new());
    assert_eq!(
        matches("flex { unknown-property: 1px; }"),
        Vec::<String>::new()
    );
    let only_important = matches("flex { margin: 1px !important; }");
    assert_eq!(only_important.len(), 1);
    entry(&only_important, "Important");
}

#[test]
fn rules_that_are_not_style_rules_are_skipped() {
    let matches = matches("@media (min-width: 1px) { flex { padding: 1px } } flex { margin: 1px }");
    assert_eq!(matches.len(), 1);
    let normal = entry(&matches, "Normal");
    assert!(normal.contains("source_order: 0 }"), "{normal}");
    assert!(normal.contains("padding: None, margin: Some"), "{normal}");
}

#[test]
fn both_halves_keep_every_selector_of_the_list() {
    let matches = matches("a, flex.item { padding: 1px; margin: 1px !important; }");
    assert_eq!(matches.len(), 2);
    for importance in ["Important", "Normal"] {
        let found = entry(&matches, importance);
        assert!(
            found.contains("specificity: Specificity { id: 0, class: 1, element: 1 }"),
            "{found}"
        );
    }
}

#[test]
fn a_stylesheet_lightningcss_rejects_is_a_parser_error() {
    let result = LightningCssAdapter::new().parse_stylesheet(
        StyleSheetName::new("test").unwrap(),
        "flex { color: red; } @import 'late.css';",
    );
    assert!(matches!(
        result,
        Err(StylingError::ParserError(message)) if message.contains("@import rules must precede")
    ));
}
