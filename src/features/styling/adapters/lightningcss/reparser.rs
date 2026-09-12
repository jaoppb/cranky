use super::properties::parse_property_list;
use crate::features::styling::domain::ComputedStyle;
use crate::features::styling::ports::PropertyReparser;
use lightningcss::rules::CssRule;
use lightningcss::stylesheet::{ParserOptions, StyleSheet};

/// Re-parses one declaration by wrapping it in a throwaway rule.
///
/// Runs it back through the ordinary property pipeline — the same one
/// every other declaration in a sheet already goes through. The selector
/// doesn't matter; nothing here ever matches it against an element.
pub struct LightningPropertyReparser;

impl PropertyReparser for LightningPropertyReparser {
    fn reparse(&self, name: &str, value: &str) -> ComputedStyle {
        let css = format!("x{{{name}:{value};}}");
        let Ok(sheet) = StyleSheet::parse(&css, ParserOptions::default()) else {
            return ComputedStyle::default();
        };
        let Some(CssRule::Style(rule)) = sheet.rules.0.first() else {
            return ComputedStyle::default();
        };
        parse_property_list(&rule.declarations.declarations)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reparses_a_substituted_declaration() {
        let reparser = LightningPropertyReparser;
        let style = reparser.reparse("background-color", "#7aa2f7");
        assert!(style.background().is_some());
    }

    #[test]
    fn test_invalid_declaration_yields_default() {
        let reparser = LightningPropertyReparser;
        let style = reparser.reparse("not-a-real-property", "nonsense(((");
        assert_eq!(style, ComputedStyle::default());
    }
}
