//! Detects `em`/`rem` (and `calc()`) values on length properties, so the cascade can resolve
//! them against the right font size instead of the 16px guess.
use super::calc::{calc_from_length_percentage_or_auto, calc_from_max_size, calc_from_size};
use super::relative_spacing::apply_spacing;
use crate::features::styling::domain::{DeclaredLength, DeclaredLengths};
use lightningcss::properties::Property;
use lightningcss::properties::font::FontSize as LightningFontSize;
use lightningcss::properties::size::{MaxSize, Size};
use lightningcss::stylesheet::PrinterOptions;
use lightningcss::traits::ToCss;
use lightningcss::values::length::LengthPercentageOrAuto;

/// The declared `font-size`, if it used `em`/`rem`.
///
/// A `Pixels`/`%`/absolute keyword result means `color_font::apply_font_size`
/// already got it right, so there's nothing for the caller to correct.
/// Kept separate from `detect_relative_lengths` below: font-size resolves
/// against the *parent's* size, not the element's own, so `DeclaredStyle`
/// treats it differently.
#[must_use]
pub(crate) fn detect_relative_font_size(props: &[Property<'_>]) -> Option<DeclaredLength> {
    for prop in props {
        if let Property::FontSize(LightningFontSize::Length(length)) = prop {
            let css_text = length
                .to_css_string(PrinterOptions::default())
                .unwrap_or_default();
            if let Some(len @ (DeclaredLength::Ems(_) | DeclaredLength::Rem(_))) =
                DeclaredLength::parse(&css_text)
            {
                return Some(len);
            }
        }
    }
    None
}

/// Scans a rule's raw declarations for `em`/`rem` on any of the twenty
/// length-bearing properties `DeclaredLengths` tracks.
///
/// Everything but `font-size`, handled above since it needs different —
/// parent-relative, not own-element-relative — resolution.
///
/// Mirrors the matching in `box_model`/`flex`/`grid::container`'s ordinary
/// property appliers property-for-property; those keep writing a
/// (possibly wrong, until this pass corrects it) pixel guess to
/// `ComputedStyle`, so non-relative values need no changes here at all.
#[must_use]
pub(crate) fn detect_relative_lengths(props: &[Property<'_>]) -> DeclaredLengths {
    let mut lengths = DeclaredLengths::default();
    for prop in props {
        apply_sizing(&mut lengths, prop);
        apply_border(&mut lengths, prop);
        apply_spacing(&mut lengths, prop);
        apply_flex(&mut lengths, prop);
        apply_gap_longhand(&mut lengths, prop);
    }
    lengths
}

/// The declared length, if `value`'s serialized form used `em`/`rem` — a
/// `Pixels`/`Percent`/`Auto` result means the ordinary pipeline already got it
/// right, so there's nothing for this pass to do.
pub(super) fn relative(value: &impl ToCss) -> Option<DeclaredLength> {
    let css_text = value.to_css_string(PrinterOptions::default()).ok()?;
    match DeclaredLength::parse(&css_text)? {
        len @ (DeclaredLength::Ems(_) | DeclaredLength::Rem(_)) => Some(len),
        // `DeclaredLength::parse` (string-based) never actually produces
        // `Calc` — only `calc.rs`'s typed tree walk does — but the match
        // still has to be exhaustive.
        DeclaredLength::Pixels(_)
        | DeclaredLength::Percent(_)
        | DeclaredLength::Auto
        | DeclaredLength::Calc { .. } => None,
    }
}

fn apply_sizing(lengths: &mut DeclaredLengths, prop: &Property<'_>) {
    match prop {
        Property::Width(size) => set_size(lengths, size, DeclaredLengths::set_width),
        Property::Height(size) => set_size(lengths, size, DeclaredLengths::set_height),
        Property::MinWidth(size) => set_size(lengths, size, DeclaredLengths::set_min_width),
        Property::MinHeight(size) => set_size(lengths, size, DeclaredLengths::set_min_height),
        Property::MaxWidth(size) => set_max_size(lengths, size, DeclaredLengths::set_max_width),
        Property::MaxHeight(size) => set_max_size(lengths, size, DeclaredLengths::set_max_height),
        _ => {}
    }
}

/// `calc()` is checked ahead of the plain em/rem/percent/auto scan: a
/// `calc()` value never matches that string-based grammar (it doesn't end
/// in a bare unit suffix), so the two checks are naturally mutually
/// exclusive — this is just the order that avoids wasting the (cheap)
/// string check on values already known to be `calc()`.
fn set_size(
    lengths: &mut DeclaredLengths,
    size: &Size,
    setter: fn(&mut DeclaredLengths, DeclaredLength),
) {
    if let Some(declared) = calc_from_size(size).or_else(|| relative(size)) {
        setter(lengths, declared);
    }
}

/// As `set_size`, for `max-width`/`max-height`'s separate `MaxSize` type.
fn set_max_size(
    lengths: &mut DeclaredLengths,
    size: &MaxSize,
    setter: fn(&mut DeclaredLengths, DeclaredLength),
) {
    if let Some(declared) = calc_from_max_size(size).or_else(|| relative(size)) {
        setter(lengths, declared);
    }
}

fn apply_border(lengths: &mut DeclaredLengths, prop: &Property<'_>) {
    match prop {
        Property::BorderRadius(radius, _) => {
            relative(&radius.top_left.0).inspect(|&declared| lengths.set_border_radius(declared))
        }
        Property::BorderWidth(width) => {
            relative(&width.top).inspect(|&declared| lengths.set_border_size(declared))
        }
        Property::Border(border) => {
            relative(&border.width).inspect(|&declared| lengths.set_border_size(declared))
        }
        _ => None,
    };
}

fn apply_flex(lengths: &mut DeclaredLengths, prop: &Property<'_>) {
    match prop {
        Property::FlexBasis(basis, _) => set_flex_basis(lengths, basis),
        Property::Flex(flex, _) => set_flex_basis(lengths, &flex.basis),
        Property::Gap(gap) => {
            if let Some(declared) = relative(&gap.row) {
                lengths.set_gap(declared);
                lengths.set_row_gap(declared);
            }
            relative(&gap.column).inspect(|&declared| lengths.set_column_gap(declared));
        }
        _ => {}
    }
}

fn set_flex_basis(lengths: &mut DeclaredLengths, value: &LengthPercentageOrAuto) {
    if let Some(declared) = calc_from_length_percentage_or_auto(value).or_else(|| relative(value)) {
        lengths.set_flex_basis(declared);
    }
}

/// `column-gap`/`row-gap` longhands: like the ordinary applier in
/// `grid::container`, matched by serialized property name rather than a
/// typed variant.
fn apply_gap_longhand(lengths: &mut DeclaredLengths, prop: &Property<'_>) {
    let Ok(full) = prop.to_css_string(false, PrinterOptions::default()) else {
        return;
    };
    let (name, val) = full
        .split_once(':')
        .map_or(("", full.as_str()), |(property, value)| {
            (property.trim(), value.trim())
        });
    let Some(declared @ (DeclaredLength::Ems(_) | DeclaredLength::Rem(_))) =
        DeclaredLength::parse(val)
    else {
        return;
    };
    match name {
        "column-gap" => lengths.set_column_gap(declared),
        "row-gap" => lengths.set_row_gap(declared),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::styling::adapters::lightningcss::LightningCssAdapter;
    use crate::features::styling::domain::cascade::declared_lengths_apply::FontSizeBases;
    use crate::features::styling::domain::{ComputedStyle, CssLength, StyleSheetName};
    use crate::features::styling::ports::CssParserPort;
    use crate::shared::config::domain::FontSize;
    use lightningcss::rules::CssRule;
    use lightningcss::stylesheet::{ParserOptions, StyleSheet};

    fn declarations(css: &str) -> Vec<Property<'_>> {
        let sheet = StyleSheet::parse(css, ParserOptions::default()).unwrap();
        let CssRule::Style(style_rule) = sheet.rules.0.first().unwrap() else {
            panic!("expected a style rule");
        };
        style_rule.declarations.declarations.clone()
    }

    #[test]
    fn test_detects_em_width_ignores_px_height() {
        let props = declarations("bar { width: 2em; height: 10px; }");
        let lengths = detect_relative_lengths(&props);

        let mut computed = ComputedStyle::default();
        lengths.apply(
            &mut computed,
            FontSizeBases::new(FontSize::new(10.0), FontSize::new(14.0)),
        );

        assert_eq!(computed.width(), Some(CssLength::Pixels(20.0)));
        // height was px, not relative: nothing pending, untouched here.
        assert_eq!(computed.height(), None);
    }

    #[test]
    fn test_detects_rem_padding_per_side() {
        let props = declarations("bar { padding: 1rem 0 0 0; }");
        let lengths = detect_relative_lengths(&props);

        let mut computed = ComputedStyle::default();
        lengths.apply(
            &mut computed,
            FontSizeBases::new(FontSize::new(10.0), FontSize::new(20.0)),
        );

        assert_eq!(
            computed
                .padding()
                .map(crate::features::layout_engine::domain::BoxMargin::top),
            Some(20.0)
        );
    }

    #[test]
    fn test_parser_still_accepts_these_rules() {
        // Sanity check that the adapter as a whole still parses fine with
        // em/rem present, independent of this module's own logic.
        let parser = LightningCssAdapter::new();
        assert!(
            parser
                .parse_stylesheet(
                    StyleSheetName::new("test").unwrap(),
                    "bar { padding: 1em 0.5rem; gap: 1em; }",
                )
                .is_ok()
        );
    }
}
