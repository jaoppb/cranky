use super::super::computed_style::ComputedStyle;
use super::declared_length::DeclaredLength;
use super::declared_lengths::DeclaredLengths;
use super::inherited::{DEFAULT_FONT_SIZE, InheritedStyle};
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

/// One rule's declarations before inheritance and `em`/`rem` are resolved.
///
/// Every property except the four inheritable ones and the twenty
/// length-bearing ones (see `DeclaredLengths`) collapses straight to
/// `ComputedStyle`'s `Option<T>` shape exactly as before (`rest`) — nothing
/// else in this codebase's grammar ever inherits, gets explicitly reset, or
/// carries a font-relative unit.
#[derive(Debug, Clone, Default)]
pub struct DeclaredStyle {
    rest: ComputedStyle,
    color: Cascade<DrawingColor>,
    accent_color: Cascade<DrawingColor>,
    font_family: Cascade<Arc<str>>,
    font_size: Cascade<DeclaredLength>,
    lengths: DeclaredLengths,
}

impl DeclaredStyle {
    /// Builds a `DeclaredStyle` from one rule's already-parsed
    /// `ComputedStyle`, any CSS-wide keywords the adapter found on the four
    /// inheritable properties, `font-size`'s own declaration if it used a
    /// font-relative unit, and every other length property's declaration
    /// that did.
    #[must_use]
    pub fn from_parts(
        computed: ComputedStyle,
        keywords: InheritableKeywords,
        font_size_declared: Option<DeclaredLength>,
        lengths: DeclaredLengths,
    ) -> Self {
        let color = cascade_from(keywords.color, computed.color().cloned());
        let accent_color = cascade_from(keywords.accent_color, computed.accent_color().cloned());
        let font_family = cascade_from(
            keywords.font_family,
            computed.font_family().map(|f| Arc::from(f.as_str())),
        );
        // The ordinary (non-relative) property pipeline already resolved
        // font-size — assuming a fixed base if it happened to be em/rem.
        // `font_size_declared` overrides that assumption with the real
        // declaration whenever one was found.
        let font_size_value = font_size_declared.or_else(|| {
            computed
                .font_size()
                .map(|fs| DeclaredLength::Px(fs.value()))
        });
        let font_size = cascade_from(keywords.font_size, font_size_value);
        Self {
            rest: computed,
            color,
            accent_color,
            font_family,
            font_size,
            lengths,
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
            self.font_size = other.font_size;
        }
        self.lengths.merge_with(&other.lengths);
    }

    /// Resolves inheritance and `em`/`rem`: a property left `NotDeclared`
    /// (or explicitly `inherit`) takes the parent's value; `initial` resets
    /// it regardless of what the parent had; a declared `Value` always
    /// wins. Font-size resolves first, since every other length's `em` is
    /// relative to *this* element's own font-size, not its parent's.
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

        let own_font_size = resolve_font_size(self.font_size, inherited);
        if let Some(size) = own_font_size {
            computed.set_font_size(size);
        }
        // Every other length's `em` base: this element's own font-size when
        // resolved, else the same fallback the renderer already uses when
        // painting an element with no font-size anywhere in its ancestry.
        let em_base = own_font_size.unwrap_or(FontSize::new(DEFAULT_FONT_SIZE));
        self.lengths
            .apply(&mut computed, em_base, inherited.root_font_size());

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

/// `font-size` gets its own resolution instead of the generic `resolve`
/// above: even a *declared* `em`/`rem` value still needs arithmetic against
/// the parent's font-size and the root base, which `resolve`'s plain
/// pass-through doesn't do.
fn resolve_font_size(
    cascade: Cascade<DeclaredLength>,
    inherited: &InheritedStyle,
) -> Option<FontSize> {
    let parent_or_default = || {
        inherited
            .font_size()
            .copied()
            .unwrap_or(FontSize::new(DEFAULT_FONT_SIZE))
    };
    match cascade {
        Cascade::Value(DeclaredLength::Px(v)) => Some(FontSize::new(v)),
        Cascade::Value(DeclaredLength::Percent(v)) => {
            Some(FontSize::new(v / 100.0 * parent_or_default().value()))
        }
        Cascade::Value(DeclaredLength::Em(v)) => {
            Some(FontSize::new(v * parent_or_default().value()))
        }
        Cascade::Value(DeclaredLength::Rem(v)) => {
            Some(FontSize::new(v * inherited.root_font_size().value()))
        }
        Cascade::Value(DeclaredLength::Auto) | Cascade::NotDeclared | Cascade::Inherit => {
            inherited.font_size().copied()
        }
        Cascade::Initial => Some(FontSize::new(DEFAULT_FONT_SIZE)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn color(hex: &str) -> DrawingColor {
        DrawingColor::parse(hex).unwrap()
    }

    fn declared(computed: ComputedStyle, keywords: InheritableKeywords) -> DeclaredStyle {
        DeclaredStyle::from_parts(computed, keywords, None, DeclaredLengths::default())
    }

    #[test]
    fn test_not_declared_falls_back_to_inherited() {
        let style = declared(ComputedStyle::default(), InheritableKeywords::default());
        let mut parent = ComputedStyle::default();
        parent.set_color(color("#ffffff"));
        let inherited = InheritedStyle::default().descend(&parent);

        let computed = style.collapse(&inherited);
        assert_eq!(computed.color(), Some(&color("#ffffff")));
    }

    #[test]
    fn test_declared_value_wins_over_inherited() {
        let mut own = ComputedStyle::default();
        own.set_color(color("#000000"));
        let style = declared(own, InheritableKeywords::default());

        let mut parent = ComputedStyle::default();
        parent.set_color(color("#ffffff"));
        let inherited = InheritedStyle::default().descend(&parent);

        let computed = style.collapse(&inherited);
        assert_eq!(computed.color(), Some(&color("#000000")));
    }

    #[test]
    fn test_explicit_initial_does_not_inherit() {
        let style = declared(
            ComputedStyle::default(),
            InheritableKeywords {
                color: Some(CssWideKeyword::Initial),
                ..Default::default()
            },
        );
        let mut parent = ComputedStyle::default();
        parent.set_color(color("#ffffff"));
        let inherited = InheritedStyle::default().descend(&parent);

        let computed = style.collapse(&inherited);
        assert_eq!(computed.color(), None);
    }

    #[test]
    fn test_explicit_inherit_keyword_pulls_parent_value() {
        let style = declared(
            ComputedStyle::default(),
            InheritableKeywords {
                color: Some(CssWideKeyword::Inherit),
                ..Default::default()
            },
        );
        let mut parent = ComputedStyle::default();
        parent.set_color(color("#123456"));
        let inherited = InheritedStyle::default().descend(&parent);

        let computed = style.collapse(&inherited);
        assert_eq!(computed.color(), Some(&color("#123456")));
    }

    #[test]
    fn test_merge_with_prefers_last_declared_over_not_declared() {
        let mut base = declared(
            {
                let mut c = ComputedStyle::default();
                c.set_color(color("#000000"));
                c
            },
            InheritableKeywords::default(),
        );
        let override_style = declared(ComputedStyle::default(), InheritableKeywords::default());

        base.merge_with(&override_style);
        let computed = base.collapse(&InheritedStyle::default());
        // override_style declared nothing, so the earlier value survives.
        assert_eq!(computed.color(), Some(&color("#000000")));
    }

    #[test]
    fn test_em_font_size_resolves_against_parent() {
        let style = DeclaredStyle::from_parts(
            ComputedStyle::default(),
            InheritableKeywords::default(),
            Some(DeclaredLength::Em(1.5)),
            DeclaredLengths::default(),
        );
        let mut parent = ComputedStyle::default();
        parent.set_font_size(FontSize::new(20.0));
        let inherited = InheritedStyle::default().descend(&parent);

        let computed = style.collapse(&inherited);
        assert_eq!(computed.font_size().map(|fs| fs.value()), Some(30.0));
    }

    #[test]
    fn test_rem_font_size_resolves_against_root_not_parent() {
        let style = DeclaredStyle::from_parts(
            ComputedStyle::default(),
            InheritableKeywords::default(),
            Some(DeclaredLength::Rem(2.0)),
            DeclaredLengths::default(),
        );
        let mut parent = ComputedStyle::default();
        parent.set_font_size(FontSize::new(20.0));
        let inherited = InheritedStyle::default().descend(&parent);

        let computed = style.collapse(&inherited);
        // Root base is DEFAULT_FONT_SIZE (14.0), not the parent's 20.0.
        assert_eq!(computed.font_size().map(|fs| fs.value()), Some(28.0));
    }

    #[test]
    fn test_em_width_resolves_against_own_font_size() {
        let mut own = ComputedStyle::default();
        own.set_font_size(FontSize::new(10.0));
        let mut lengths = DeclaredLengths::default();
        lengths.set_width(DeclaredLength::Em(2.0));
        let style = DeclaredStyle::from_parts(own, InheritableKeywords::default(), None, lengths);

        let computed = style.collapse(&InheritedStyle::default());
        assert_eq!(
            computed.width(),
            Some(crate::features::styling::domain::CssLength::Px(20.0))
        );
    }
}
