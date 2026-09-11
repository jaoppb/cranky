use super::super::computed_style::ComputedStyle;
use super::inherited::InheritedStyle;
use super::value::{Cascade, CssWideKeyword};
use crate::shared::config::domain::{FontFamily, FontSize};
use crate::shared::primitives::color::DrawingColor;
use std::sync::Arc;

/// The four properties this renderer treats as naturally inherited, each
/// possibly carrying an explicit CSS-wide keyword instead of a normal
/// value.
///
/// Produced by the adapter from a rule's raw declarations — `inherit`/
/// `initial` only exist as syntax at that layer — and consumed by
/// `DeclaredStyle::from_parts`.
#[derive(Debug, Clone, Copy, Default)]
pub struct InheritableKeywords {
    pub color: Option<CssWideKeyword>,
    pub accent_color: Option<CssWideKeyword>,
    pub font_family: Option<CssWideKeyword>,
    pub font_size: Option<CssWideKeyword>,
}

/// One rule's declarations before inheritance is resolved.
///
/// Every property except the four inheritable ones collapses straight to
/// `ComputedStyle`'s `Option<T>` shape exactly as before (`rest`) — nothing
/// in this codebase ever inherits or is explicitly reset for those, so a
/// full four-state `Cascade<T>` there would model states that can never
/// occur. The four that do inherit get real `Cascade<T>` tracking so
/// `collapse` can tell "not declared" (falls back to the parent) apart from
/// an explicit `initial` (does not, even though nothing else declared it).
#[derive(Debug, Clone, Default)]
pub struct DeclaredStyle {
    rest: ComputedStyle,
    color: Cascade<DrawingColor>,
    accent_color: Cascade<DrawingColor>,
    font_family: Cascade<Arc<str>>,
    font_size: Cascade<FontSize>,
}

impl DeclaredStyle {
    /// Builds a `DeclaredStyle` from one rule's already-parsed
    /// `ComputedStyle` plus any CSS-wide keywords the adapter found on the
    /// four inheritable properties.
    #[must_use]
    pub fn from_parts(computed: ComputedStyle, keywords: InheritableKeywords) -> Self {
        let color = cascade_from(keywords.color, computed.color().cloned());
        let accent_color = cascade_from(keywords.accent_color, computed.accent_color().cloned());
        let font_family = cascade_from(
            keywords.font_family,
            computed.font_family().map(|f| Arc::from(f.as_str())),
        );
        let font_size = cascade_from(keywords.font_size, computed.font_size());
        Self {
            rest: computed,
            color,
            accent_color,
            font_family,
            font_size,
        }
    }

    pub fn merge_with(&mut self, other: &Self) {
        self.rest.merge_with(&other.rest);
        if !matches!(other.color, Cascade::NotDeclared) {
            self.color = other.color.clone();
        }
        if !matches!(other.accent_color, Cascade::NotDeclared) {
            self.accent_color = other.accent_color.clone();
        }
        if !matches!(other.font_family, Cascade::NotDeclared) {
            self.font_family = other.font_family.clone();
        }
        if !matches!(other.font_size, Cascade::NotDeclared) {
            self.font_size = other.font_size.clone();
        }
    }

    /// Resolves inheritance: a property left `NotDeclared` (or explicitly
    /// `inherit`) takes the parent's value; `initial` resets it regardless
    /// of what the parent had; a declared `Value` always wins.
    #[must_use]
    pub fn collapse(self, inherited: &InheritedStyle) -> ComputedStyle {
        let mut computed = self.rest;
        if let Some(color) = resolve(self.color, inherited.color()) {
            computed.set_color(color);
        }
        if let Some(accent) = resolve(self.accent_color, inherited.accent_color()) {
            computed.set_accent_color(accent);
        }
        if let Some(family) = resolve(self.font_family, inherited.font_family()) {
            computed.set_font_family(FontFamily::new(family.to_string()));
        }
        if let Some(size) = resolve(self.font_size, inherited.font_size()) {
            computed.set_font_size(size);
        }
        computed
    }
}

fn cascade_from<T>(keyword: Option<CssWideKeyword>, value: Option<T>) -> Cascade<T> {
    match keyword {
        Some(CssWideKeyword::Inherit) => Cascade::Inherit,
        Some(CssWideKeyword::Initial) => Cascade::Initial,
        None => value.map_or(Cascade::NotDeclared, Cascade::Value),
    }
}

fn resolve<T: Clone>(cascade: Cascade<T>, inherited: Option<&T>) -> Option<T> {
    match cascade {
        Cascade::Value(v) => Some(v),
        Cascade::NotDeclared | Cascade::Inherit => inherited.cloned(),
        Cascade::Initial => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn color(hex: &str) -> DrawingColor {
        DrawingColor::parse(hex).unwrap()
    }

    #[test]
    fn test_not_declared_falls_back_to_inherited() {
        let declared =
            DeclaredStyle::from_parts(ComputedStyle::default(), InheritableKeywords::default());
        let mut parent = ComputedStyle::default();
        parent.set_color(color("#ffffff"));
        let inherited = InheritedStyle::from_computed(&parent);

        let computed = declared.collapse(&inherited);
        assert_eq!(computed.color(), Some(&color("#ffffff")));
    }

    #[test]
    fn test_declared_value_wins_over_inherited() {
        let mut own = ComputedStyle::default();
        own.set_color(color("#000000"));
        let declared = DeclaredStyle::from_parts(own, InheritableKeywords::default());

        let mut parent = ComputedStyle::default();
        parent.set_color(color("#ffffff"));
        let inherited = InheritedStyle::from_computed(&parent);

        let computed = declared.collapse(&inherited);
        assert_eq!(computed.color(), Some(&color("#000000")));
    }

    #[test]
    fn test_explicit_initial_does_not_inherit() {
        let declared = DeclaredStyle::from_parts(
            ComputedStyle::default(),
            InheritableKeywords {
                color: Some(CssWideKeyword::Initial),
                ..Default::default()
            },
        );
        let mut parent = ComputedStyle::default();
        parent.set_color(color("#ffffff"));
        let inherited = InheritedStyle::from_computed(&parent);

        let computed = declared.collapse(&inherited);
        assert_eq!(computed.color(), None);
    }

    #[test]
    fn test_explicit_inherit_keyword_pulls_parent_value() {
        let declared = DeclaredStyle::from_parts(
            ComputedStyle::default(),
            InheritableKeywords {
                color: Some(CssWideKeyword::Inherit),
                ..Default::default()
            },
        );
        let mut parent = ComputedStyle::default();
        parent.set_color(color("#123456"));
        let inherited = InheritedStyle::from_computed(&parent);

        let computed = declared.collapse(&inherited);
        assert_eq!(computed.color(), Some(&color("#123456")));
    }

    #[test]
    fn test_merge_with_prefers_last_declared_over_not_declared() {
        let mut base = DeclaredStyle::from_parts(
            {
                let mut c = ComputedStyle::default();
                c.set_color(color("#000000"));
                c
            },
            InheritableKeywords::default(),
        );
        let override_style =
            DeclaredStyle::from_parts(ComputedStyle::default(), InheritableKeywords::default());

        base.merge_with(&override_style);
        let computed = base.collapse(&InheritedStyle::default());
        // override_style declared nothing, so the earlier value survives.
        assert_eq!(computed.color(), Some(&color("#000000")));
    }
}
