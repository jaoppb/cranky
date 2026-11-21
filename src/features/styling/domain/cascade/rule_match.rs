//! A rule that matched an element inside one stylesheet, before its layer and sheet position
//! are attached.
use super::declared_style::DeclaredStyle;
use super::fold::MatchedRule;
use super::priority::{Importance, Layer, RulePriority};
use super::specificity::Specificity;

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
    style: DeclaredStyle,
}

impl RuleMatch {
    #[must_use]
    pub(crate) const fn new(
        importance: Importance,
        specificity: Specificity,
        source_order: u32,
        style: DeclaredStyle,
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
    pub(crate) fn into_matched(self, layer: Layer, sheet_index: u32) -> MatchedRule {
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
