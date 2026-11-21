//! The compiled selector model and the rule entries a parsed stylesheet holds.
use super::super::cascade::{DeclaredStyle, Importance};
use super::super::grid_types::PseudoClass;

/// Relationship between two consecutive steps in a compiled selector.
///
/// Mirrors the CSS combinator grammar; kept as a domain type so the domain
/// selector model has no dependency on any parsing crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Combinator {
    /// ` ` — any ancestor.
    Descendant,
    /// `>` — direct parent.
    Child,
    /// `+` — immediately preceding sibling.
    NextSibling,
    /// `~` — any preceding sibling.
    LaterSibling,
}

#[derive(Debug, Clone)]
pub(crate) struct RuleEntry {
    pub(crate) selectors: Vec<CompiledSelector>,
    pub(crate) style: DeclaredStyle,
    pub(crate) importance: Importance,
}

#[derive(Debug, Clone)]
pub(crate) struct CompiledSelector {
    pub(crate) steps: Vec<SelectorStep>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct SelectorStep {
    pub(crate) tag: Option<String>,
    pub(crate) id: Option<String>,
    pub(crate) classes: Vec<String>,
    pub(crate) pseudo_classes: Vec<PseudoClass>,
    pub(crate) negations: Vec<CompiledSelector>,
    pub(crate) combinator: Option<Combinator>,
}
