//! A rule that matched an element inside one stylesheet, before its layer and sheet position
//! are attached.
use super::declared_style::DeclaredStyle;
use super::fold::MatchedRule;
use super::priority::{CascadeOrigin, RulePriority, RuleRank};

/// One rule's match against an element, before cross-sheet layer and sheet
/// position are known.
///
/// Produced by `ParsedStyleSheetPort::matching_rules`, which only knows
/// about its own sheet's rules and their source order.
#[derive(Debug, Clone)]
pub struct RuleMatch {
    rank: RuleRank,
    style: DeclaredStyle,
}

impl RuleMatch {
    #[must_use]
    pub(crate) const fn new(rank: RuleRank, style: DeclaredStyle) -> Self {
        Self { rank, style }
    }

    /// Attaches this match's position in the cascade — its layer, and which
    /// sheet (among however many are composed together) it came from —
    /// producing a fully-ordered `MatchedRule` ready to fold.
    #[must_use]
    pub(crate) fn into_matched(self, origin: CascadeOrigin) -> MatchedRule {
        MatchedRule::new(RulePriority::new(self.rank, origin), self.style)
    }
}
