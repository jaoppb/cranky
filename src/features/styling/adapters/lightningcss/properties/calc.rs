use crate::features::styling::domain::DeclaredLength;
use lightningcss::properties::size::{MaxSize, Size};
use lightningcss::values::calc::{Calc, MathFunction};
use lightningcss::values::length::{LengthPercentage, LengthPercentageOrAuto, LengthValue};

/// A `calc()` expression's contribution while walking lightningcss's typed
/// tree, before `em`/`rem` are resolved against a font size — one
/// coefficient per term kind, since `calc()` for a `<length-percentage>` is
/// always a linear combination of these (CSS never multiplies two lengths
/// together).
#[derive(Debug, Clone, Copy, Default)]
struct Accum {
    percent: f32,
    px: f32,
    em: f32,
    rem: f32,
}

impl Accum {
    const fn add(self, other: Self) -> Self {
        Self {
            percent: self.percent + other.percent,
            px: self.px + other.px,
            em: self.em + other.em,
            rem: self.rem + other.rem,
        }
    }
}

/// The `DeclaredLength::Calc` for `size`, if it's a `calc()` expression —
/// `None` means the ordinary (non-calc) detection/parsing path handles it.
#[must_use]
pub fn calc_from_size(size: &Size) -> Option<DeclaredLength> {
    let Size::LengthPercentage(lp) = size else {
        return None;
    };
    calc_from_length_percentage(lp)
}

/// As `calc_from_size`, for `max-width`/`max-height`'s separate `MaxSize`
/// (lightningcss models `none` as its own keyword there, rather than
/// reusing `Size`'s `auto`).
#[must_use]
pub fn calc_from_max_size(size: &MaxSize) -> Option<DeclaredLength> {
    let MaxSize::LengthPercentage(lp) = size else {
        return None;
    };
    calc_from_length_percentage(lp)
}

/// As `calc_from_size`, for `flex-basis`'s `LengthPercentageOrAuto`.
#[must_use]
pub fn calc_from_length_percentage_or_auto(
    value: &LengthPercentageOrAuto,
) -> Option<DeclaredLength> {
    let LengthPercentageOrAuto::LengthPercentage(lp) = value else {
        return None;
    };
    calc_from_length_percentage(lp)
}

fn calc_from_length_percentage(lp: &LengthPercentage) -> Option<DeclaredLength> {
    let LengthPercentage::Calc(calc) = lp else {
        return None;
    };
    let accum = walk_calc(calc, 1.0);
    Some(DeclaredLength::Calc {
        percent: accum.percent,
        px: accum.px,
        em: accum.em,
        rem: accum.rem,
    })
}

/// Walks `Sum`/`Product`/nested `calc()` — the only shapes lightningcss's
/// parser produces for addition, subtraction, multiplication and division
/// (the latter two are already normalized into `Sum`/`Product` at parse
/// time). The `calc(...)` function call
/// itself round-trips through `Calc::Function(MathFunction::Calc(inner))` —
/// unwrapped here rather than treated as opaque. `min()`/`max()`/`clamp()`/
/// `round()` and friends (`MathFunction`'s other variants) aren't walked —
/// nothing in this renderer's supported grammar needs them, so they
/// contribute zero rather than failing the whole expression.
fn walk_calc(calc: &Calc<LengthPercentage>, scale: f32) -> Accum {
    match calc {
        Calc::Value(v) => walk_value(v, scale),
        Calc::Number(_) => Accum::default(),
        Calc::Sum(a, b) => walk_calc(a, scale).add(walk_calc(b, scale)),
        Calc::Product(num, inner) => walk_calc(inner, scale * *num),
        Calc::Function(f) => match f.as_ref() {
            MathFunction::Calc(inner) => walk_calc(inner, scale),
            _ => Accum::default(),
        },
    }
}

fn walk_value(v: &LengthPercentage, scale: f32) -> Accum {
    match v {
        LengthPercentage::Dimension(length_value) => walk_length_value(length_value, scale),
        LengthPercentage::Percentage(p) => Accum {
            percent: p.0 * 100.0 * scale,
            ..Accum::default()
        },
        LengthPercentage::Calc(nested) => walk_calc(nested, scale),
    }
}

/// Only the units `DeclaredLength` itself supports (`px`/`em`/`rem`) — every
/// other CSS length unit (`in`/`cm`/`mm`/`pt`/`pc`, and the `ex`/`ch`/`cap`/
/// `ic`/`lh` family and their root-relative variants) contributes zero
/// rather than being silently mis-scaled as pixels.
fn walk_length_value(lv: &LengthValue, scale: f32) -> Accum {
    match lv {
        LengthValue::Px(v) => Accum {
            px: v * scale,
            ..Accum::default()
        },
        LengthValue::Em(v) => Accum {
            em: v * scale,
            ..Accum::default()
        },
        LengthValue::Rem(v) => Accum {
            rem: v * scale,
            ..Accum::default()
        },
        _ => Accum::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lightningcss::properties::Property;
    use lightningcss::rules::CssRule;
    use lightningcss::stylesheet::{ParserOptions, StyleSheet};

    fn width_value(css: &str) -> Size {
        let sheet = StyleSheet::parse(css, ParserOptions::default()).unwrap();
        let CssRule::Style(style_rule) = sheet.rules.0.first().unwrap() else {
            panic!("expected a style rule");
        };
        let Property::Width(size) = style_rule.declarations.declarations.first().unwrap() else {
            panic!("expected a width declaration");
        };
        size.clone()
    }

    #[test]
    fn test_calc_absolute_only_no_percent() {
        let size = width_value("bar { width: calc(1rem + 8px); }");
        let declared = calc_from_size(&size).unwrap();
        assert_eq!(
            declared,
            DeclaredLength::Calc {
                percent: 0.0,
                px: 8.0,
                em: 0.0,
                rem: 1.0,
            }
        );
    }

    #[test]
    fn test_calc_percent_minus_absolute() {
        let size = width_value("bar { width: calc(100% - 2rem); }");
        let declared = calc_from_size(&size).unwrap();
        assert_eq!(
            declared,
            DeclaredLength::Calc {
                percent: 100.0,
                px: 0.0,
                em: 0.0,
                rem: -2.0,
            }
        );
    }

    #[test]
    fn test_non_calc_width_is_not_detected_as_calc() {
        let size = width_value("bar { width: 50%; }");
        assert!(calc_from_size(&size).is_none());
    }
}
