use super::declared_length::DeclaredLength;
use super::inherited::{DEFAULT_FONT_SIZE, InheritedStyle};
use super::value::{Cascade, CssWideKeyword};
use crate::shared::config::domain::FontSize;

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
        Cascade::Value(v) => Some(v),
        Cascade::NotDeclared | Cascade::Inherit => inherited.cloned(),
        Cascade::Initial => None,
    }
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
