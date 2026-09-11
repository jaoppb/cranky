/// The four ways a cascaded value can resolve, per the CSS cascade's
/// defaulting rules: declared to a real value, explicitly reset via a
/// CSS-wide keyword, or simply not mentioned by any matching rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Cascade<T> {
    #[default]
    NotDeclared,
    Value(T),
    Inherit,
    Initial,
}

/// A CSS-wide keyword (<https://drafts.csswg.org/css-cascade-5/#defaulting-keywords>)
/// detected in place of a property's normal value grammar.
///
/// Only `inherit` and `initial` are modeled: `unset` collapses to `inherit`
/// for the properties this renderer treats as inheritable (all four
/// naturally are, per spec), and `revert`/`revert-layer` have no earlier
/// origin or layer to roll back to in a two-layer, single-origin cascade.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CssWideKeyword {
    Inherit,
    Initial,
}
