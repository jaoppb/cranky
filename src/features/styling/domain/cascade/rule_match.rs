use super::fold::MatchedRule;
use super::priority::{Importance, Layer, RulePriority};
use super::specificity::Specificity;
use crate::features::styling::domain::computed_style::ComputedStyle;

/// One rule's match against an element, before cross-sheet layer and sheet
/// position are known.
///
/// Produced by `ParsedStyleSheetPort::matching_rules`, which only knows
/// about its own sheet's rules and their source order.
#[derive(Debug, Clone)]
pub struct RuleMatch {
    importance: Importance,
    specificity: Specificity,
    source_order: u32,
    style: ComputedStyle,
}

impl RuleMatch {
    #[must_use]
    pub const fn new(
        importance: Importance,
        specificity: Specificity,
        source_order: u32,
        style: ComputedStyle,
    ) -> Self {
        Self {
            importance,
            specificity,
            source_order,
            style,
        }
    }

    /// Attaches this match's position in the cascade — its layer, and which
    /// sheet (among however many are composed together) it came from —
    /// producing a fully-ordered `MatchedRule` ready to fold.
    #[must_use]
    pub fn into_matched(self, layer: Layer, sheet_index: u32) -> MatchedRule {
        let priority = RulePriority::new(
            self.importance,
            layer,
            self.specificity,
            sheet_index,
            self.source_order,
        );
        MatchedRule::new(priority, self.style)
    }
}
