use crate::features::styling::domain::{
    ComputedStyle, ElementQuery, InheritedStyle, Layer, RuleEntry, RuleMatch, StyleSheetName,
    fold_matches, matches_selector, selector_specificity,
};
use crate::features::styling::ports::ParsedStyleSheetPort;

pub struct LightningParsedStyleSheet {
    name: StyleSheetName,
    rules: Vec<RuleEntry>,
}

impl LightningParsedStyleSheet {
    #[must_use]
    pub const fn new(name: StyleSheetName, rules: Vec<RuleEntry>) -> Self {
        Self { name, rules }
    }
}

impl ParsedStyleSheetPort for LightningParsedStyleSheet {
    fn name(&self) -> &StyleSheetName {
        &self.name
    }

    fn resolve_style(&self, query: &ElementQuery, inherited: &InheritedStyle) -> ComputedStyle {
        // Single sheet: layer and sheet index are irrelevant since every
        // match shares them, so importance/specificity/source-order alone
        // decide the outcome.
        let matched = self
            .matching_rules(query)
            .into_iter()
            .map(|m| m.into_matched(Layer::BASE, 0))
            .collect();
        let result = fold_matches(matched).collapse(inherited);

        tracing::trace!(
            stylesheet = %self.name.as_str(),
            tag = %query.tag(),
            id = ?query.id().map(crate::features::styling::domain::ElementId::as_str),
            classes = ?query.classes().iter().map(crate::features::styling::domain::ClassName::as_str).collect::<Vec<_>>(),
            has_bg = result.background().is_some(),
            has_color = result.color().is_some(),
            "Resolved style for query"
        );

        result
    }

    fn matching_rules(&self, query: &ElementQuery) -> Vec<RuleMatch> {
        self.rules
            .iter()
            .enumerate()
            .filter_map(|(index, rule)| {
                // A rule can list several comma-separated selectors sharing
                // one declaration block; only matching ones count, and among
                // those the most specific governs (per-selector specificity,
                // same declarations either way).
                let specificity = rule
                    .selectors
                    .iter()
                    .filter(|sel| matches_selector(sel, query))
                    .map(selector_specificity)
                    .max()?;
                let source_order = u32::try_from(index).unwrap_or(u32::MAX);
                Some(RuleMatch::new(
                    rule.importance,
                    specificity,
                    source_order,
                    rule.style.clone(),
                ))
            })
            .collect()
    }
}
