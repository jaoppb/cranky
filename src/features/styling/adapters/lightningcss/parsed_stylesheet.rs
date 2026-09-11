use crate::features::styling::domain::{
    ComputedStyle, ElementQuery, RuleEntry, StyleSheetName, matches_selector,
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

    fn resolve_style(&self, query: &ElementQuery) -> ComputedStyle {
        let mut result = ComputedStyle::default();

        for rule in &self.rules {
            let matches = rule
                .selectors
                .iter()
                .any(|sel| matches_selector(sel, query));
            if matches {
                result.merge_with(&rule.style);
            }
        }

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
}
