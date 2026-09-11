use super::parsed_stylesheet::LightningParsedStyleSheet;
use super::properties::parse_declarations;
use super::selector::compile_selector;
use crate::features::styling::domain::{RuleEntry, StyleSheetName, StylingError};
use crate::features::styling::ports::{CssParserPort, ParsedStyleSheetPort};
use lightningcss::rules::CssRule;
use lightningcss::stylesheet::{ParserOptions, StyleSheet};

pub struct LightningCssAdapter;

impl LightningCssAdapter {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl Default for LightningCssAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl CssParserPort for LightningCssAdapter {
    fn parse_stylesheet(
        &self,
        name: StyleSheetName,
        css_source: &str,
    ) -> Result<Box<dyn ParsedStyleSheetPort>, StylingError> {
        tracing::debug!(stylesheet = %name.as_str(), len = css_source.len(), "Parsing CSS stylesheet");
        let stylesheet = StyleSheet::parse(css_source, ParserOptions::default()).map_err(|e| {
            tracing::error!(stylesheet = %name.as_str(), error = ?e, "Failed to parse CSS stylesheet");
            StylingError::ParserError(e.to_string())
        })?;

        let mut rule_entries = Vec::new();

        for rule in &stylesheet.rules.0 {
            if let CssRule::Style(style_rule) = rule {
                let computed = parse_declarations(&style_rule.declarations);
                let mut selectors = Vec::new();
                for sel in &style_rule.selectors.0 {
                    selectors.push(compile_selector(sel));
                }
                rule_entries.push(RuleEntry {
                    selectors,
                    style: computed,
                });
            }
        }

        tracing::debug!(stylesheet = %name.as_str(), rule_count = rule_entries.len(), "CSS stylesheet parsed successfully");

        Ok(Box::new(LightningParsedStyleSheet::new(
            name,
            rule_entries,
        )))
    }
}
