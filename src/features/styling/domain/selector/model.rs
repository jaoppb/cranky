use super::super::cascade::{DeclaredStyle, Importance};
use super::super::grid_types::PseudoClass;

/// Relationship between two consecutive steps in a compiled selector.
///
/// Mirrors the CSS combinator grammar; kept as a domain type so the domain
/// selector model has no dependency on any parsing crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Combinator {
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
pub struct RuleEntry {
    pub selectors: Vec<CompiledSelector>,
    pub style: DeclaredStyle,
    pub importance: Importance,
}

#[derive(Debug, Clone)]
pub struct CompiledSelector {
    pub steps: Vec<SelectorStep>,
}

#[derive(Debug, Clone, Default)]
pub struct SelectorStep {
    pub tag: Option<String>,
    pub id: Option<String>,
    pub classes: Vec<String>,
    pub pseudo_classes: Vec<PseudoClass>,
    pub negations: Vec<CompiledSelector>,
    pub combinator: Option<Combinator>,
}
