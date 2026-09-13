use super::super::values::CssLength;
use crate::shared::config::domain::FontSize;

/// A length as declared in CSS, before `em`/`rem` are resolved against a
/// font size.
///
/// Mirrors `CssLength`'s shape (`Percent`/`Auto` included) since every
/// length-bearing property in this renderer's grammar accepts the same
/// unit set, even properties (gap, padding, border) whose extraction path
/// today never actually produces `Percent`/`Auto`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DeclaredLength {
    Px(f32),
    Percent(f32),
    Em(f32),
    Rem(f32),
    Auto,
    /// A `calc()` expression, folded into a linear combination of its four
    /// possible term kinds while walking lightningcss's typed calc tree
    /// (adapter-side, since only that tree — not this codebase's own
    /// string-based unit parsing — represents `calc()` at all). `em`/`rem`
    /// still need a font-size to resolve, exactly like the plain `Em`/`Rem`
    /// variants, so this stays unresolved until `resolve`.
    Calc {
        percent: f32,
        px: f32,
        em: f32,
        rem: f32,
    },
}

impl DeclaredLength {
    /// Parses a serialized CSS length/percentage, preserving `em`/`rem`
    /// instead of resolving them — the counterpart to
    /// `box_model::utils::parse_size_str`, which resolves immediately
    /// against a fixed assumption. Order matters: `rem` must be checked
    /// before `em`, since `"2rem"` also ends in `"em"`.
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        let trimmed = s.trim();
        if trimmed.eq_ignore_ascii_case("auto") {
            return Some(Self::Auto);
        }
        if let Some(pct) = trimmed.strip_suffix('%') {
            return pct.trim().parse::<f32>().ok().map(Self::Percent);
        }
        if let Some(num) = trimmed.strip_suffix("rem") {
            return num.trim().parse::<f32>().ok().map(Self::Rem);
        }
        if let Some(num) = trimmed.strip_suffix("em") {
            return num.trim().parse::<f32>().ok().map(Self::Em);
        }
        if let Some(num) = trimmed.strip_suffix("px") {
            return num.trim().parse::<f32>().ok().map(Self::Px);
        }
        trimmed.parse::<f32>().ok().map(Self::Px)
    }

    /// Resolves `em`/`rem` into concrete pixels, using `font_size` — the
    /// font size `em` is relative to (the element's own, except for
    /// `font-size` itself, where CSS defines `em` as relative to the
    /// *parent's*) — and `root_font_size` for `rem`.
    #[must_use]
    pub fn resolve(self, font_size: FontSize, root_font_size: FontSize) -> CssLength {
        match self {
            Self::Px(v) => CssLength::Px(v),
            Self::Percent(v) => CssLength::Percent(v),
            Self::Em(v) => CssLength::Px(v * font_size.value()),
            Self::Rem(v) => CssLength::Px(v * root_font_size.value()),
            Self::Auto => CssLength::Auto,
            Self::Calc {
                percent,
                px,
                em,
                rem,
            } => {
                let px_total =
                    rem.mul_add(root_font_size.value(), em.mul_add(font_size.value(), px));
                // No percent term: the whole expression is already a single
                // concrete pixel value, so it never needs the layout
                // engine's second pass — the same "absolute calc() is free"
                // property that made em/rem itself free of layout changes.
                if percent == 0.0 {
                    CssLength::Px(px_total)
                } else {
                    CssLength::Calc {
                        percent,
                        px: px_total,
                    }
                }
            }
        }
    }

    /// As `resolve`, for the properties that only ever carry a pixel
    /// scalar (gap, border, padding, margin) — `Percent`/`Auto` never
    /// actually occur there today, but degrade to `0.0`/the raw percentage
    /// number rather than panicking if a future grammar change allows it.
    /// A `calc()` percent term degrades the same way: these properties have
    /// no containing-block basis to resolve it against, so only the
    /// absolute part survives.
    #[must_use]
    pub fn resolve_scalar(self, font_size: FontSize, root_font_size: FontSize) -> f32 {
        match self.resolve(font_size, root_font_size) {
            CssLength::Px(v) | CssLength::Percent(v) => v,
            CssLength::Auto => 0.0,
            CssLength::Calc { px, .. } => px,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_rem_before_em_suffix_ambiguity() {
        assert_eq!(
            DeclaredLength::parse("2rem"),
            Some(DeclaredLength::Rem(2.0))
        );
        assert_eq!(
            DeclaredLength::parse("1.5em"),
            Some(DeclaredLength::Em(1.5))
        );
    }

    #[test]
    fn test_parse_percent_auto_px() {
        assert_eq!(
            DeclaredLength::parse("50%"),
            Some(DeclaredLength::Percent(50.0))
        );
        assert_eq!(DeclaredLength::parse("auto"), Some(DeclaredLength::Auto));
        assert_eq!(DeclaredLength::parse("8px"), Some(DeclaredLength::Px(8.0)));
        assert_eq!(DeclaredLength::parse("8"), Some(DeclaredLength::Px(8.0)));
    }

    #[test]
    fn test_resolve_em_uses_font_size_rem_uses_root() {
        let font_size = FontSize::new(20.0);
        let root = FontSize::new(10.0);
        assert_eq!(
            DeclaredLength::Em(2.0).resolve(font_size, root),
            CssLength::Px(40.0)
        );
        assert_eq!(
            DeclaredLength::Rem(2.0).resolve(font_size, root),
            CssLength::Px(20.0)
        );
    }

    #[test]
    fn test_calc_without_percent_collapses_to_plain_px() {
        let font_size = FontSize::new(20.0);
        let root = FontSize::new(10.0);
        let calc = DeclaredLength::Calc {
            percent: 0.0,
            px: 8.0,
            em: 0.0,
            rem: 1.0,
        };
        // 8px + 1rem(10px) = 18px, and no percent term means no layout-time
        // resolution is needed — same shape as a plain Px value.
        assert_eq!(calc.resolve(font_size, root), CssLength::Px(18.0));
    }

    #[test]
    fn test_calc_with_percent_stays_symbolic_for_layout() {
        let font_size = FontSize::new(20.0);
        let root = FontSize::new(10.0);
        let calc = DeclaredLength::Calc {
            percent: 100.0,
            px: -16.0,
            em: 0.0,
            rem: 1.0,
        };
        // calc(100% - 1rem - 16px) -> percent stays 100, px folds to -6.
        assert_eq!(
            calc.resolve(font_size, root),
            CssLength::Calc {
                percent: 100.0,
                px: -6.0
            }
        );
    }
}
