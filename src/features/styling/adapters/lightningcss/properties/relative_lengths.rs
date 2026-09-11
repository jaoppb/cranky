use crate::features::styling::domain::{DeclaredLength, DeclaredLengths};
use lightningcss::properties::Property;
use lightningcss::properties::font::FontSize as LightningFontSize;
use lightningcss::stylesheet::PrinterOptions;
use lightningcss::traits::ToCss;

/// The declared `font-size`, if it used `em`/`rem`.
///
/// A `Px`/`%`/absolute keyword result means `color_font::apply_font_size`
/// already got it right, so there's nothing for the caller to correct.
/// Kept separate from `detect_relative_lengths` below: font-size resolves
/// against the *parent's* size, not the element's own, so `DeclaredStyle`
/// treats it differently.
#[must_use]
pub fn detect_relative_font_size(props: &[Property]) -> Option<DeclaredLength> {
    for prop in props {
        if let Property::FontSize(LightningFontSize::Length(l)) = prop {
            let s = l
                .to_css_string(PrinterOptions::default())
                .unwrap_or_default();
            if let Some(len @ (DeclaredLength::Em(_) | DeclaredLength::Rem(_))) =
                DeclaredLength::parse(&s)
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
pub fn detect_relative_lengths(props: &[Property]) -> DeclaredLengths {
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
/// `Px`/`Percent`/`Auto` result means the ordinary pipeline already got it
/// right, so there's nothing for this pass to do.
fn relative(value: &impl ToCss) -> Option<DeclaredLength> {
    let s = value.to_css_string(PrinterOptions::default()).ok()?;
    match DeclaredLength::parse(&s)? {
        len @ (DeclaredLength::Em(_) | DeclaredLength::Rem(_)) => Some(len),
        DeclaredLength::Px(_) | DeclaredLength::Percent(_) | DeclaredLength::Auto => None,
    }
}

fn apply_sizing(lengths: &mut DeclaredLengths, prop: &Property) {
    match prop {
        Property::Width(v) => relative(v).inspect(|&v| lengths.set_width(v)),
        Property::Height(v) => relative(v).inspect(|&v| lengths.set_height(v)),
        Property::MinWidth(v) => relative(v).inspect(|&v| lengths.set_min_width(v)),
        Property::MinHeight(v) => relative(v).inspect(|&v| lengths.set_min_height(v)),
        Property::MaxWidth(v) => relative(v).inspect(|&v| lengths.set_max_width(v)),
        Property::MaxHeight(v) => relative(v).inspect(|&v| lengths.set_max_height(v)),
        _ => None,
    };
}

fn apply_border(lengths: &mut DeclaredLengths, prop: &Property) {
    match prop {
        Property::BorderRadius(radius, _) => {
            relative(&radius.top_left.0).inspect(|&v| lengths.set_border_radius(v))
        }
        Property::BorderWidth(width) => {
            relative(&width.top).inspect(|&v| lengths.set_border_size(v))
        }
        Property::Border(border) => {
            relative(&border.width).inspect(|&v| lengths.set_border_size(v))
        }
        _ => None,
    };
}

fn apply_spacing(lengths: &mut DeclaredLengths, prop: &Property) {
    match prop {
        Property::Padding(p) => {
            relative(&p.top).inspect(|&v| lengths.set_padding_top(v));
            relative(&p.right).inspect(|&v| lengths.set_padding_right(v));
            relative(&p.bottom).inspect(|&v| lengths.set_padding_bottom(v));
            relative(&p.left).inspect(|&v| lengths.set_padding_left(v));
        }
        Property::PaddingTop(v) => {
            relative(v).inspect(|&v| lengths.set_padding_top(v));
        }
        Property::PaddingRight(v) => {
            relative(v).inspect(|&v| lengths.set_padding_right(v));
        }
        Property::PaddingBottom(v) => {
            relative(v).inspect(|&v| lengths.set_padding_bottom(v));
        }
        Property::PaddingLeft(v) => {
            relative(v).inspect(|&v| lengths.set_padding_left(v));
        }
        Property::Margin(m) => {
            relative(&m.top).inspect(|&v| lengths.set_margin_top(v));
            relative(&m.right).inspect(|&v| lengths.set_margin_right(v));
            relative(&m.bottom).inspect(|&v| lengths.set_margin_bottom(v));
            relative(&m.left).inspect(|&v| lengths.set_margin_left(v));
        }
        Property::MarginTop(v) => {
            relative(v).inspect(|&v| lengths.set_margin_top(v));
        }
        Property::MarginRight(v) => {
            relative(v).inspect(|&v| lengths.set_margin_right(v));
        }
        Property::MarginBottom(v) => {
            relative(v).inspect(|&v| lengths.set_margin_bottom(v));
        }
        Property::MarginLeft(v) => {
            relative(v).inspect(|&v| lengths.set_margin_left(v));
        }
        _ => {}
    }
}

fn apply_flex(lengths: &mut DeclaredLengths, prop: &Property) {
    match prop {
        Property::FlexBasis(v, _) => {
            relative(v).inspect(|&v| lengths.set_flex_basis(v));
        }
        Property::Flex(flex, _) => {
            relative(&flex.basis).inspect(|&v| lengths.set_flex_basis(v));
        }
        Property::Gap(gap) => {
            if let Some(v) = relative(&gap.row) {
                lengths.set_gap(v);
                lengths.set_row_gap(v);
            }
            relative(&gap.column).inspect(|&v| lengths.set_column_gap(v));
        }
        _ => {}
    }
}

/// `column-gap`/`row-gap` longhands: like the ordinary applier in
/// `grid::container`, matched by serialized property name rather than a
/// typed variant.
fn apply_gap_longhand(lengths: &mut DeclaredLengths, prop: &Property) {
    let Ok(full) = prop.to_css_string(false, PrinterOptions::default()) else {
        return;
    };
    let (name, val) = full
        .split_once(':')
        .map_or(("", full.as_str()), |(k, v)| (k.trim(), v.trim()));
    let Some(declared @ (DeclaredLength::Em(_) | DeclaredLength::Rem(_))) =
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
        lengths.apply(&mut computed, FontSize::new(10.0), FontSize::new(14.0));

        assert_eq!(computed.width(), Some(CssLength::Px(20.0)));
        // height was px, not relative: nothing pending, untouched here.
        assert_eq!(computed.height(), None);
    }

    #[test]
    fn test_detects_rem_padding_per_side() {
        let props = declarations("bar { padding: 1rem 0 0 0; }");
        let lengths = detect_relative_lengths(&props);

        let mut computed = ComputedStyle::default();
        lengths.apply(&mut computed, FontSize::new(10.0), FontSize::new(20.0));

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
