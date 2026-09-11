use super::parsed_stylesheet::LightningParsedStyleSheet;
use super::properties::{
    detect_inheritable_keywords, detect_relative_font_size, detect_relative_lengths,
    parse_property_list,
};
use super::selector::compile_selector;
use crate::features::styling::domain::{
    ComputedStyle, DeclaredStyle, Importance, InheritableKeywords, RuleEntry, StyleSheetName,
    StylingError,
};
use crate::features::styling::ports::{CssParserPort, ParsedStyleSheetPort};
use lightningcss::rules::CssRule;
use lightningcss::stylesheet::{ParserOptions, StyleSheet};

/// True if any of the four inheritable properties carried an explicit
/// CSS-wide keyword, even when nothing else in the declaration list parsed
/// to a normal value (e.g. a rule containing only `color: inherit;`).
const fn has_keyword(keywords: InheritableKeywords) -> bool {
    keywords.color.is_some()
        || keywords.accent_color.is_some()
        || keywords.font_family.is_some()
        || keywords.font_size.is_some()
}

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
                let mut selectors = Vec::new();
                for sel in &style_rule.selectors.0 {
                    selectors.push(compile_selector(sel));
                }

                // Normal and !important declarations are cascaded separately
                // (importance outranks specificity), so they become distinct
                // rule entries sharing the same selectors.
                let normal_style = parse_property_list(&style_rule.declarations.declarations);
                let normal_keywords =
                    detect_inheritable_keywords(&style_rule.declarations.declarations);
                let normal_font_size =
                    detect_relative_font_size(&style_rule.declarations.declarations);
                let normal_lengths = detect_relative_lengths(&style_rule.declarations.declarations);

                let important_style =
                    parse_property_list(&style_rule.declarations.important_declarations);
                let important_keywords =
                    detect_inheritable_keywords(&style_rule.declarations.important_declarations);
                let important_font_size =
                    detect_relative_font_size(&style_rule.declarations.important_declarations);
                let important_lengths =
                    detect_relative_lengths(&style_rule.declarations.important_declarations);

                if important_style != ComputedStyle::default() || has_keyword(important_keywords) {
                    rule_entries.push(RuleEntry {
                        selectors: selectors.clone(),
                        style: DeclaredStyle::from_parts(
                            important_style,
                            important_keywords,
                            important_font_size,
                            important_lengths,
                        ),
                        importance: Importance::Important,
                    });
                }
                if normal_style != ComputedStyle::default() || has_keyword(normal_keywords) {
                    rule_entries.push(RuleEntry {
                        selectors,
                        style: DeclaredStyle::from_parts(
                            normal_style,
                            normal_keywords,
                            normal_font_size,
                            normal_lengths,
                        ),
                        importance: Importance::Normal,
                    });
                }
            }
        }

        tracing::debug!(stylesheet = %name.as_str(), rule_count = rule_entries.len(), "CSS stylesheet parsed successfully");

        Ok(Box::new(LightningParsedStyleSheet::new(name, rule_entries)))
    }
}
