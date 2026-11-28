//! Parses a stylesheet with lightningcss into rule entries, keeping each rule's normal and
//! `!important` declarations, CSS-wide keywords, relative lengths and custom properties.
use super::parsed_stylesheet::LightningParsedStyleSheet;
use super::properties::{
    detect_custom_property_declarations, detect_inheritable_keywords,
    detect_pending_var_properties, detect_relative_font_size, detect_relative_lengths,
    parse_property_list,
};
use super::selector::compile_selector;
use crate::features::styling::domain::{
    ComputedStyle, DeclaredStyle, DeclaredValues, FontRelativeLengths, Importance,
    InheritableKeywords, RuleEntry, StyleSheetName, StylingError, VarDeclarations,
};
use crate::features::styling::ports::{CssParserPort, ParsedStyleSheetPort};
use lightningcss::error::{Error, ParserError};
use lightningcss::properties::Property;
use lightningcss::rules::CssRule;
use lightningcss::rules::style::StyleRule;
use lightningcss::stylesheet::{ParserOptions, StyleSheet};
use std::collections::HashMap;

/// True if any of the four inheritable properties carried an explicit
/// CSS-wide keyword, even when nothing else in the declaration list parsed
/// to a normal value (e.g. a rule containing only `color: inherit;`).
const fn has_keyword(keywords: InheritableKeywords) -> bool {
    keywords.color.is_some()
        || keywords.accent_color.is_some()
        || keywords.font_family.is_some()
        || keywords.font_size.is_some()
        || keywords.text_align.is_some()
}

/// True if a rule's declarations were entirely `--name: ...`/`var(...)`
/// properties — those produce nothing in the ordinary `ComputedStyle` and
/// carry no CSS-wide keyword, so without this check they'd look empty and
/// get silently dropped.
fn has_custom_or_pending(
    custom: &HashMap<String, String>,
    pending: &HashMap<String, String>,
) -> bool {
    !custom.is_empty() || !pending.is_empty()
}

/// Parses CSS source into a [`ParsedStyleSheetPort`]; the registry uses one to build every
/// module's stylesheets.
#[derive(Debug)]
pub struct LightningCssAdapter;

impl LightningCssAdapter {
    /// Creates the parser; it holds no state, so one instance serves every stylesheet.
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
        log_parsing(&name, css_source);
        let stylesheet = StyleSheet::parse(css_source, ParserOptions::default())
            .map_err(|error| parse_failure(&name, &error))?;

        let mut rule_entries = Vec::new();
        for style_rule in stylesheet.rules.0.iter().filter_map(as_style_rule) {
            push_rule_entries(style_rule, &mut rule_entries);
        }

        log_parsed(&name, rule_entries.len());
        Ok(Box::new(LightningParsedStyleSheet::new(name, rule_entries)))
    }
}

/// The style rule `rule` is, or `None` for an at-rule or any other kind of rule.
const fn as_style_rule<'rule, 'i>(rule: &'rule CssRule<'i>) -> Option<&'rule StyleRule<'i>> {
    let CssRule::Style(style_rule) = rule else {
        return None;
    };
    Some(style_rule)
}

/// Appends the entries of one style rule: its `!important` half, then its normal half, each
/// only if it declares something.
fn push_rule_entries(style_rule: &StyleRule<'_>, rule_entries: &mut Vec<RuleEntry>) {
    let selectors: Vec<_> = style_rule
        .selectors
        .0
        .iter()
        .map(compile_selector)
        .collect();
    // Normal and !important declarations are cascaded separately
    // (importance outranks specificity), so they become distinct
    // rule entries sharing the same selectors. The normal half is read
    // first, as it always was, and entered last.
    let normal = declared_style(&style_rule.declarations.declarations);
    let important = declared_style(&style_rule.declarations.important_declarations);
    if let Some(style) = important {
        rule_entries.push(RuleEntry {
            selectors: selectors.clone(),
            style,
            importance: Importance::Important,
        });
    }
    if let Some(style) = normal {
        rule_entries.push(RuleEntry {
            selectors,
            style,
            importance: Importance::Normal,
        });
    }
}

/// What one half of a rule declares, or `None` when it declares no value, no CSS-wide keyword,
/// no custom property and no `var()` use.
fn declared_style(declarations: &[Property<'_>]) -> Option<DeclaredStyle> {
    let style = parse_property_list(declarations);
    let keywords = detect_inheritable_keywords(declarations);
    let font_size = detect_relative_font_size(declarations);
    let lengths = detect_relative_lengths(declarations);
    let custom = detect_custom_property_declarations(declarations);
    let pending_vars = detect_pending_var_properties(declarations);
    let declares_something = style != ComputedStyle::default()
        || has_keyword(keywords)
        || has_custom_or_pending(&custom, &pending_vars);
    declares_something.then(|| {
        DeclaredStyle::from_parts(
            DeclaredValues::new(style, keywords),
            FontRelativeLengths::new(font_size, lengths),
            VarDeclarations::new(custom, pending_vars),
        )
    })
}

/// Logs that the sheet `name` is about to be parsed from `css_source`.
///
/// Kept out of line to keep its caller under `cognitive_complexity`.
fn log_parsing(name: &StyleSheetName, css_source: &str) {
    tracing::debug!(stylesheet = %name.as_str(), len = css_source.len(), "Parsing CSS stylesheet");
}

/// Logs that lightningcss rejected the sheet `name` and turns its `error` into a parser error.
fn parse_failure(name: &StyleSheetName, error: &Error<ParserError<'_>>) -> StylingError {
    tracing::error!(stylesheet = %name.as_str(), error = ?error, "Failed to parse CSS stylesheet");
    StylingError::ParserError(error.to_string())
}

/// Logs that the sheet `name` parsed into `rule_count` rule entries.
///
/// Kept out of line to keep its caller under `cognitive_complexity`.
fn log_parsed(name: &StyleSheetName, rule_count: usize) {
    tracing::debug!(stylesheet = %name.as_str(), rule_count = rule_count, "CSS stylesheet parsed successfully");
}
