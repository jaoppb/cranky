//! Helpers that turn a cascaded value and the inherited one into the final value, with
//! `font-size` resolved against the parent's size.
use super::super::computed_style::ComputedStyle;
use super::declared_length::DeclaredLength;
use super::inherited::{DEFAULT_FONT_SIZE, InheritedStyle};
use super::value::{Cascade, CssWideKeyword};
use crate::shared::config::domain::FontSize;
use std::collections::HashMap;
use std::sync::Arc;

/// Small resolution helpers `DeclaredStyle::collapse` uses, split out to
/// keep `declared_style.rs` under the 200 LOC production file limit.
pub(super) fn cascade_from<T>(keyword: Option<CssWideKeyword>, value: Option<T>) -> Cascade<T> {
    match keyword {
        Some(CssWideKeyword::Inherit) => Cascade::Inherit,
        Some(CssWideKeyword::Initial) => Cascade::Initial,
        None => value.map_or(Cascade::NotDeclared, Cascade::Value),
    }
}

pub(super) fn resolve<T: Clone>(cascade: Cascade<T>, inherited: Option<&T>) -> Option<T> {
    match cascade {
        Cascade::Value(value) => Some(value),
        Cascade::NotDeclared | Cascade::Inherit => inherited.cloned(),
        Cascade::Initial => None,
    }
}

/// `font-size`'s cascade for one rule, unless a CSS-wide keyword overrides it.
///
/// The ordinary (non-relative) property pipeline already resolved font-size into `computed` —
/// assuming a fixed base if it happened to be em/rem. The font-relative declaration (`relative`)
/// overrides that assumption with the real declaration whenever one was found.
pub(super) fn font_size_cascade(
    keyword: Option<CssWideKeyword>,
    relative: Option<DeclaredLength>,
    computed: &ComputedStyle,
) -> Cascade<DeclaredLength> {
    let declared = relative.or_else(|| {
        computed
            .font_size()
            .map(|font_size| DeclaredLength::Pixels(font_size.value()))
    });
    cascade_from(keyword, declared)
}

/// `font-size` gets its own resolution instead of the generic `resolve`
/// above.
///
/// Even a *declared* `em`/`rem` value still needs arithmetic against the
/// parent's font-size and the root base, which `resolve`'s plain
/// pass-through doesn't do.
pub(super) fn resolve_font_size(
    cascade: Cascade<DeclaredLength>,
    inherited: &InheritedStyle,
) -> Option<FontSize> {
    match cascade {
        Cascade::Value(length) => font_size_from_length(length, inherited),
        Cascade::NotDeclared | Cascade::Inherit => inherited.font_size().copied(),
        Cascade::Initial => Some(FontSize::new(DEFAULT_FONT_SIZE)),
    }
}

/// The font-size a declared length gives: `%` and `em` scale the parent's font-size (or the
/// default when no ancestor has one), `rem` the root's, and `auto` keeps the parent's.
fn font_size_from_length(length: DeclaredLength, inherited: &InheritedStyle) -> Option<FontSize> {
    let parent = inherited
        .font_size()
        .copied()
        .unwrap_or(FontSize::new(DEFAULT_FONT_SIZE))
        .value();
    match length {
        DeclaredLength::Pixels(pixels) => Some(FontSize::new(pixels)),
        DeclaredLength::Percent(percentage) => Some(FontSize::new(percentage / 100.0 * parent)),
        DeclaredLength::Ems(ems) => Some(FontSize::new(ems * parent)),
        DeclaredLength::Rem(root_ems) => {
            Some(FontSize::new(root_ems * inherited.root_font_size().value()))
        }
        DeclaredLength::Calc {
            percent,
            px,
            ems,
            rem,
        } => {
            // font-size is the one length where a calc() percent term is
            // resolvable at style time rather than needing the layout
            // engine's second pass: both `%` and `em` are defined as
            // relative to the *parent's* font-size, which is already known
            // here.
            let px_total = (percent / 100.0 + ems).mul_add(parent, px);
            Some(FontSize::new(
                rem.mul_add(inherited.root_font_size().value(), px_total),
            ))
        }
        DeclaredLength::Auto => inherited.font_size().copied(),
    }
}

/// The custom properties this element's children resolve `var()` against: the parent's,
/// overridden by whatever this element declared (`declared`) — cheap when it declared none
/// (an `Arc` clone, no copy).
pub(super) fn own_custom_properties(
    declared: HashMap<String, String>,
    inherited: &InheritedStyle,
) -> Arc<HashMap<String, String>> {
    if declared.is_empty() {
        Arc::clone(inherited.custom_properties())
    } else {
        let mut merged = (**inherited.custom_properties()).clone();
        merged.extend(declared);
        Arc::new(merged)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::styling::domain::ComputedStyle;
    use std::collections::HashMap;
    use std::sync::Arc;

    /// What a child of an element whose computed font-size is `parent` inherits.
    fn under_parent(parent: Option<f32>) -> InheritedStyle {
        let mut style = ComputedStyle::default();
        if let Some(size) = parent {
            style.set_font_size(FontSize::new(size));
        }
        InheritedStyle::default().descend(&style, Arc::new(HashMap::new()))
    }

    fn font_size(cascade: Cascade<DeclaredLength>, parent: Option<f32>) -> Option<f32> {
        resolve_font_size(cascade, &under_parent(parent)).map(FontSize::value)
    }

    #[test]
    fn test_pixels_ignore_the_parent() {
        let declared = Cascade::Value(DeclaredLength::Pixels(9.0));
        assert_eq!(font_size(declared, Some(20.0)), Some(9.0));
    }

    #[test]
    fn test_percent_scales_the_parent_or_the_default() {
        let declared = Cascade::Value(DeclaredLength::Percent(150.0));
        assert_eq!(font_size(declared, Some(20.0)), Some(30.0));
        assert_eq!(font_size(declared, None), Some(21.0));
    }

    #[test]
    fn test_ems_scale_the_parent_or_the_default() {
        let declared = Cascade::Value(DeclaredLength::Ems(2.0));
        assert_eq!(font_size(declared, Some(20.0)), Some(40.0));
        assert_eq!(font_size(declared, None), Some(28.0));
    }

    #[test]
    fn test_rem_scales_the_root_not_the_parent() {
        let declared = Cascade::Value(DeclaredLength::Rem(2.0));
        assert_eq!(font_size(declared, Some(20.0)), Some(28.0));
    }

    #[test]
    fn test_calc_sums_percent_and_ems_of_the_parent_pixels_and_rem_of_the_root() {
        let declared = Cascade::Value(DeclaredLength::Calc {
            percent: 50.0,
            px: 2.0,
            ems: 0.25,
            rem: 1.0,
        });
        // (50% + 0.25em) of 20 = 15, + 2px, + 1rem of 14.
        assert_eq!(font_size(declared, Some(20.0)), Some(31.0));
        // Without a parent both relative terms use the default 14: 10.5 + 2 + 14.
        assert_eq!(font_size(declared, None), Some(26.5));
    }

    #[test]
    fn test_auto_not_declared_and_inherit_take_the_parent_size() {
        for cascade in [
            Cascade::Value(DeclaredLength::Auto),
            Cascade::NotDeclared,
            Cascade::Inherit,
        ] {
            assert_eq!(font_size(cascade, Some(20.0)), Some(20.0));
            assert_eq!(font_size(cascade, None), None);
        }
    }

    #[test]
    fn test_initial_resets_to_the_default_size() {
        assert_eq!(
            font_size(Cascade::Initial, Some(20.0)),
            Some(DEFAULT_FONT_SIZE)
        );
    }

    #[test]
    fn test_cascade_from_prefers_the_keyword_over_the_value() {
        assert_eq!(
            cascade_from(Some(CssWideKeyword::Inherit), Some(1)),
            Cascade::Inherit
        );
        assert_eq!(
            cascade_from(Some(CssWideKeyword::Initial), Some(1)),
            Cascade::Initial
        );
        assert_eq!(cascade_from(None, Some(1)), Cascade::Value(1));
        assert_eq!(cascade_from::<i32>(None, None), Cascade::NotDeclared);
    }

    #[test]
    fn test_resolve_takes_the_value_then_the_parent_and_initial_clears() {
        assert_eq!(resolve(Cascade::Value(1), Some(&2)), Some(1));
        assert_eq!(resolve(Cascade::NotDeclared, Some(&2)), Some(2));
        assert_eq!(resolve(Cascade::Inherit, Some(&2)), Some(2));
        assert_eq!(resolve(Cascade::Initial, Some(&2)), None);
    }
}
