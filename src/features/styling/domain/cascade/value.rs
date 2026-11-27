//! The cascade state of one property (declared, reset by a CSS-wide keyword, or unset) and the
//! CSS-wide keywords themselves.
/// The four ways a cascaded value can resolve, per the CSS cascade's
/// defaulting rules: declared to a real value, explicitly reset via a
/// CSS-wide keyword, or simply not mentioned by any matching rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Cascade<T> {
    #[default]
    NotDeclared,
    Value(T),
    Inherit,
    Initial,
}

impl<T: Clone> Cascade<T> {
    /// Takes `later`'s state unless `later` never declared the property: a later rule's value or
    /// CSS-wide keyword replaces this one, a property it leaves out keeps this one.
    pub(crate) fn override_with(&mut self, later: &Self) {
        if !matches!(later, Self::NotDeclared) {
            self.clone_from(later);
        }
    }
}

/// A CSS-wide keyword (<https://drafts.csswg.org/css-cascade-5/#defaulting-keywords>)
/// detected in place of a property's normal value grammar.
///
/// Only `inherit` and `initial` are modeled: `unset` collapses to `inherit`
/// for the properties this renderer treats as inheritable (all four
/// naturally are, per spec), and `revert`/`revert-layer` have no earlier
/// origin or layer to roll back to in a two-layer, single-origin cascade.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CssWideKeyword {
    Inherit,
    Initial,
}
