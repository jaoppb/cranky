use lightningcss::properties::Property;
use lightningcss::properties::custom::CustomPropertyName;
use lightningcss::stylesheet::PrinterOptions;
use lightningcss::traits::ToCss;
use std::collections::HashMap;

/// Scans a rule's raw declarations for `--name: value;` custom-property
/// declarations, keyed by name (with the leading `--`).
///
/// `TokenList`'s own `to_css` is crate-private, so — like every other
/// unparsed-value read in this adapter — this serializes the whole
/// declaration and splits on the first `:` instead of reading the value in
/// isolation.
#[must_use]
pub fn detect_custom_property_declarations(props: &[Property]) -> HashMap<String, String> {
    let mut declared = HashMap::new();
    for prop in props {
        let Property::Custom(custom) = prop else {
            continue;
        };
        let CustomPropertyName::Custom(name) = &custom.name else {
            continue;
        };
        let Ok(full) = prop.to_css_string(false, PrinterOptions::default()) else {
            continue;
        };
        let value = full
            .split_once(':')
            .map_or(full.as_str(), |(_, v)| v)
            .trim()
            .to_string();
        declared.insert(
            name.to_css_string(PrinterOptions::default())
                .unwrap_or_default(),
            value,
        );
    }
    declared
}

/// Scans a rule's raw declarations for properties whose value contains
/// `var(...)`.
///
/// These fail their normal value grammar (lightningcss can't know what a
/// variable resolves to at parse time) and fall back to
/// `Property::Unparsed`, the same catch-all the CSS-wide-keyword detection
/// already reads from.
///
/// Returns each such property's name and raw value text, to be substituted
/// and re-parsed once the element's resolved custom properties are known —
/// which can't happen until `DeclaredStyle::collapse`, since substitution
/// depends on inheritance.
#[must_use]
pub fn detect_pending_var_properties(props: &[Property]) -> HashMap<String, String> {
    let mut pending = HashMap::new();
    for prop in props {
        let Property::Unparsed(unparsed) = prop else {
            continue;
        };
        let Ok(full) = prop.to_css_string(false, PrinterOptions::default()) else {
            continue;
        };
        if !full.contains("var(") {
            continue;
        }
        let value = full
            .split_once(':')
            .map_or(full.as_str(), |(_, v)| v)
            .trim()
            .to_string();
        pending.insert(unparsed.property_id.name().to_string(), value);
    }
    pending
}

#[cfg(test)]
mod tests {
    use super::*;
    use lightningcss::rules::CssRule;
    use lightningcss::stylesheet::{ParserOptions, StyleSheet};

    fn declarations(css: &str) -> Vec<Property<'_>> {
        let sheet = StyleSheet::parse(css, ParserOptions::default()).unwrap();
        let CssRule::Style(style_rule) = sheet.rules.0.first().unwrap() else {
            panic!("expected a style rule");
        };
        style_rule.declarations.declarations.clone()
    }

    #[test]
    fn test_detects_custom_property_declaration() {
        let props = declarations("bar { --bg: #1a1b26; }");
        let declared = detect_custom_property_declarations(&props);
        assert_eq!(declared.get("--bg").map(String::as_str), Some("#1a1b26"));
    }

    #[test]
    fn test_detects_pending_var_use() {
        let props = declarations("bar { background-color: var(--bg); }");
        let pending = detect_pending_var_properties(&props);
        assert_eq!(
            pending.get("background-color").map(String::as_str),
            Some("var(--bg)")
        );
    }

    #[test]
    fn test_ignores_declarations_without_var() {
        let props = declarations("bar { background-color: #ffffff; }");
        assert!(detect_pending_var_properties(&props).is_empty());
    }
}
