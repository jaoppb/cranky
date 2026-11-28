//! Pins what `DeclaredStyle::from_parts`, `merge_with` and `collapse` do with every inheritable
//! property, the font-relative lengths and the `var()` data, through their public results.
#[cfg(test)]
mod tests {
    use super::super::declared_length::DeclaredLength;
    use super::super::declared_lengths::DeclaredLengths;
    use super::super::declared_parts::{DeclaredValues, FontRelativeLengths, VarDeclarations};
    use super::super::declared_style::{DeclaredStyle, InheritableKeywords};
    use super::super::inherited::InheritedStyle;
    use super::super::value::CssWideKeyword;
    use crate::features::styling::domain::{ComputedStyle, CssLength};
    use crate::features::styling::ports::PropertyReparser;
    use crate::shared::config::domain::{FontFamily, FontSize};
    use crate::shared::primitives::TextAlign;
    use crate::shared::primitives::color::DrawingColor;
    use std::collections::HashMap;
    use std::sync::Arc;

    /// Reparses `color` and `width` (in `px`), the two properties these tests route through `var()`.
    struct ColorAndWidthReparser;

    impl PropertyReparser for ColorAndWidthReparser {
        fn reparse(&self, name: &str, value: &str) -> ComputedStyle {
            let mut style = ComputedStyle::default();
            match name {
                "color" => style.set_color(color(value.trim())),
                "width" => style.set_width(CssLength::Pixels(pixels(value))),
                _ => (),
            }
            style
        }
    }

    /// The number of pixels `value` (`8px`, or a bare `8`) declares.
    fn pixels(value: &str) -> f32 {
        value.trim().trim_end_matches("px").parse().unwrap()
    }

    fn color(hex: &str) -> DrawingColor {
        DrawingColor::parse(hex).unwrap()
    }

    /// A style with all five inheritable properties set, from `palette`'s two colors, `family`,
    /// `size` and `align`.
    fn inheritable(palette: [&str; 2], family: &str, size: f32) -> ComputedStyle {
        let mut style = ComputedStyle::default();
        style.set_color(color(palette[0]));
        style.set_accent_color(color(palette[1]));
        style.set_font_family(FontFamily::new(family.to_string()));
        style.set_font_size(FontSize::new(size));
        style
    }

    fn declared(computed: ComputedStyle, keywords: InheritableKeywords) -> DeclaredStyle {
        DeclaredStyle::from_parts(
            DeclaredValues::new(computed, keywords),
            FontRelativeLengths::default(),
            VarDeclarations::default(),
        )
    }

    fn with_vars(custom: &[(&str, &str)], pending: &[(&str, &str)]) -> DeclaredStyle {
        let map = |pairs: &[(&str, &str)]| -> HashMap<String, String> {
            pairs
                .iter()
                .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
                .collect()
        };
        DeclaredStyle::from_parts(
            DeclaredValues::new(ComputedStyle::default(), InheritableKeywords::default()),
            FontRelativeLengths::default(),
            VarDeclarations::new(map(custom), map(pending)),
        )
    }

    fn with_lengths(computed: ComputedStyle, lengths: DeclaredLengths) -> DeclaredStyle {
        DeclaredStyle::from_parts(
            DeclaredValues::new(computed, InheritableKeywords::default()),
            FontRelativeLengths::new(None, lengths),
            VarDeclarations::default(),
        )
    }

    /// What a child of an element styled `parent` inherits.
    fn under(parent: &ComputedStyle) -> InheritedStyle {
        InheritedStyle::default().descend(parent, Arc::new(HashMap::new()))
    }

    /// The parent every test collapses under: its own colors, family, size and right alignment.
    fn parent() -> InheritedStyle {
        let mut style = inheritable(["#a00000", "#b00000"], "Parent", 20.0);
        style.set_text_align(TextAlign::Right);
        under(&style)
    }

    fn collapse(style: DeclaredStyle, inherited: &InheritedStyle) -> ComputedStyle {
        style.collapse(inherited, &ColorAndWidthReparser).0
    }

    fn family(style: &ComputedStyle) -> Option<&str> {
        style.font_family().map(FontFamily::as_str)
    }

    #[test]
    fn test_from_parts_carries_every_inheritable_value_over_the_parent() {
        let mut own = inheritable(["#010000", "#020000"], "Own", 12.0);
        own.set_text_align(TextAlign::Center);

        let computed = collapse(declared(own, InheritableKeywords::default()), &parent());

        assert_eq!(computed.color(), Some(&color("#010000")));
        assert_eq!(computed.accent_color(), Some(&color("#020000")));
        assert_eq!(family(&computed), Some("Own"));
        assert_eq!(computed.font_size(), Some(FontSize::new(12.0)));
        assert_eq!(computed.text_align(), Some(TextAlign::Center));
    }

    /// Collapses a rule declaring every inheritable property, with `keywords`, under `parent()`.
    fn own_with(keywords: InheritableKeywords) -> ComputedStyle {
        let mut own = inheritable(["#010000", "#020000"], "Own", 12.0);
        own.set_text_align(TextAlign::Center);
        collapse(declared(own, keywords), &parent())
    }

    #[test]
    fn test_color_keywords_apply_to_their_own_property_only() {
        let color_inherited = own_with(InheritableKeywords {
            color: Some(CssWideKeyword::Inherit),
            ..InheritableKeywords::default()
        });
        assert_eq!(color_inherited.color(), Some(&color("#a00000")));
        assert_eq!(color_inherited.accent_color(), Some(&color("#020000")));

        let accent_inherited = own_with(InheritableKeywords {
            accent_color: Some(CssWideKeyword::Inherit),
            ..InheritableKeywords::default()
        });
        assert_eq!(accent_inherited.accent_color(), Some(&color("#b00000")));
        assert_eq!(accent_inherited.color(), Some(&color("#010000")));
    }

    #[test]
    fn test_family_and_align_keywords_apply_to_their_own_property_only() {
        let family_inherited = own_with(InheritableKeywords {
            font_family: Some(CssWideKeyword::Inherit),
            ..InheritableKeywords::default()
        });
        assert_eq!(family(&family_inherited), Some("Parent"));
        assert_eq!(family_inherited.font_size(), Some(FontSize::new(12.0)));

        let align_inherited = own_with(InheritableKeywords {
            text_align: Some(CssWideKeyword::Inherit),
            ..InheritableKeywords::default()
        });
        assert_eq!(align_inherited.text_align(), Some(TextAlign::Right));
        assert_eq!(family(&align_inherited), Some("Own"));
    }

    #[test]
    fn test_font_size_keywords_apply_to_font_size_only() {
        let size_inherited = own_with(InheritableKeywords {
            font_size: Some(CssWideKeyword::Inherit),
            ..InheritableKeywords::default()
        });
        assert_eq!(size_inherited.font_size(), Some(FontSize::new(20.0)));
        assert_eq!(size_inherited.text_align(), Some(TextAlign::Center));

        let size_reset = own_with(InheritableKeywords {
            font_size: Some(CssWideKeyword::Initial),
            ..InheritableKeywords::default()
        });
        assert_eq!(size_reset.font_size(), Some(FontSize::new(14.0)));
    }

    #[test]
    fn test_initial_keyword_without_an_own_value_does_not_inherit() {
        let mut own = ComputedStyle::default();
        own.set_color(color("#010000"));
        let keywords = InheritableKeywords {
            accent_color: Some(CssWideKeyword::Initial),
            text_align: Some(CssWideKeyword::Initial),
            ..InheritableKeywords::default()
        };

        let computed = collapse(declared(own, keywords), &parent());

        assert_eq!(computed.accent_color(), None);
        assert_eq!(computed.text_align(), None);
        assert_eq!(computed.color(), Some(&color("#010000")));
    }

    #[test]
    fn test_undeclared_inheritable_values_come_from_the_parent() {
        let computed = collapse(
            declared(ComputedStyle::default(), InheritableKeywords::default()),
            &parent(),
        );

        assert_eq!(computed.accent_color(), Some(&color("#b00000")));
        assert_eq!(family(&computed), Some("Parent"));
        assert_eq!(computed.font_size(), Some(FontSize::new(20.0)));
        assert_eq!(computed.text_align(), Some(TextAlign::Right));
    }

    #[test]
    fn test_relative_font_size_overrides_the_computed_one() {
        let mut own = ComputedStyle::default();
        own.set_font_size(FontSize::new(10.0));
        let relative = DeclaredStyle::from_parts(
            DeclaredValues::new(own.clone(), InheritableKeywords::default()),
            FontRelativeLengths::new(Some(DeclaredLength::Ems(2.0)), DeclaredLengths::default()),
            VarDeclarations::default(),
        );

        assert_eq!(
            collapse(relative, &parent()).font_size(),
            Some(FontSize::new(40.0))
        );
        let plain = declared(own, InheritableKeywords::default());
        assert_eq!(
            collapse(plain, &parent()).font_size(),
            Some(FontSize::new(10.0))
        );
    }

    #[test]
    fn test_merge_lets_the_later_rule_win_on_every_inheritable_property() {
        let mut earlier_style = inheritable(["#010000", "#020000"], "Earlier", 11.0);
        earlier_style.set_text_align(TextAlign::Left);
        let mut later_style = inheritable(["#030000", "#040000"], "Later", 13.0);
        later_style.set_text_align(TextAlign::Center);
        let mut merged = declared(earlier_style, InheritableKeywords::default());

        merged.merge_with(&declared(later_style, InheritableKeywords::default()));
        let computed = collapse(merged, &parent());

        assert_eq!(computed.color(), Some(&color("#030000")));
        assert_eq!(computed.accent_color(), Some(&color("#040000")));
        assert_eq!(family(&computed), Some("Later"));
        assert_eq!(computed.font_size(), Some(FontSize::new(13.0)));
        assert_eq!(computed.text_align(), Some(TextAlign::Center));
    }

    #[test]
    fn test_merge_keeps_every_inheritable_property_the_later_rule_leaves_out() {
        let mut earlier_style = inheritable(["#010000", "#020000"], "Earlier", 11.0);
        earlier_style.set_text_align(TextAlign::Center);
        let mut merged = declared(earlier_style, InheritableKeywords::default());

        merged.merge_with(&declared(
            ComputedStyle::default(),
            InheritableKeywords::default(),
        ));
        let computed = collapse(merged, &parent());

        assert_eq!(computed.color(), Some(&color("#010000")));
        assert_eq!(computed.accent_color(), Some(&color("#020000")));
        assert_eq!(family(&computed), Some("Earlier"));
        assert_eq!(computed.font_size(), Some(FontSize::new(11.0)));
        assert_eq!(computed.text_align(), Some(TextAlign::Center));
    }

    #[test]
    fn test_merge_takes_a_later_keyword_over_an_earlier_declaration() {
        let mut merged = declared(
            inheritable(["#010000", "#020000"], "Earlier", 11.0),
            InheritableKeywords {
                text_align: Some(CssWideKeyword::Inherit),
                ..InheritableKeywords::default()
            },
        );
        let later = declared(
            ComputedStyle::default(),
            InheritableKeywords {
                accent_color: Some(CssWideKeyword::Inherit),
                font_size: Some(CssWideKeyword::Inherit),
                text_align: Some(CssWideKeyword::Initial),
                ..InheritableKeywords::default()
            },
        );

        merged.merge_with(&later);
        let computed = collapse(merged, &parent());

        assert_eq!(computed.accent_color(), Some(&color("#b00000")));
        assert_eq!(computed.font_size(), Some(FontSize::new(20.0)));
        assert_eq!(computed.text_align(), None);
        assert_eq!(computed.color(), Some(&color("#010000")));
    }

    #[test]
    fn test_merge_merges_plain_values_and_lengths() {
        let mut earlier_style = ComputedStyle::default();
        earlier_style.set_background(color("#010000"));
        let mut earlier_lengths = DeclaredLengths::default();
        earlier_lengths.set_width(DeclaredLength::Pixels(3.0));
        earlier_lengths.set_height(DeclaredLength::Pixels(4.0));
        let mut later_lengths = DeclaredLengths::default();
        later_lengths.set_height(DeclaredLength::Pixels(5.0));
        let mut merged = with_lengths(earlier_style, earlier_lengths);

        merged.merge_with(&with_lengths(ComputedStyle::default(), later_lengths));
        let computed = collapse(merged, &parent());

        assert_eq!(computed.background(), Some(&color("#010000")));
        assert_eq!(computed.width(), Some(CssLength::Pixels(3.0)));
        assert_eq!(computed.height(), Some(CssLength::Pixels(5.0)));
    }

    #[test]
    fn test_merge_extends_custom_properties_and_pending_vars_with_the_later_rule_winning() {
        let mut merged = with_vars(
            &[("--a", "#0a0000"), ("--b", "#0b0000")],
            &[("color", "var(--a)"), ("width", "1px")],
        );

        merged.merge_with(&with_vars(&[("--b", "#0c0000")], &[("color", "var(--b)")]));
        let (computed, next) = merged.collapse(&InheritedStyle::default(), &ColorAndWidthReparser);

        assert_eq!(computed.color(), Some(&color("#0c0000")));
        assert_eq!(computed.width(), Some(CssLength::Pixels(1.0)));
        let custom = next.custom_properties();
        assert_eq!(custom.get("--a").map(String::as_str), Some("#0a0000"));
        assert_eq!(custom.get("--b").map(String::as_str), Some("#0c0000"));
    }

    #[test]
    fn test_lengths_use_the_inherited_or_default_font_size_as_em_base() {
        let mut lengths = DeclaredLengths::default();
        lengths.set_width(DeclaredLength::Ems(2.0));

        let inherited_base = collapse(
            with_lengths(ComputedStyle::default(), lengths.clone()),
            &parent(),
        );
        assert_eq!(inherited_base.width(), Some(CssLength::Pixels(40.0)));

        let default_base = collapse(
            with_lengths(ComputedStyle::default(), lengths),
            &InheritedStyle::default(),
        );
        assert_eq!(default_base.width(), Some(CssLength::Pixels(28.0)));
    }

    #[test]
    fn test_own_custom_properties_override_the_inherited_ones_for_vars_and_children() {
        let mut inherited_custom = HashMap::new();
        inherited_custom.insert("--a".to_string(), "#0a0000".to_string());
        inherited_custom.insert("--b".to_string(), "#0b0000".to_string());
        let inherited = InheritedStyle::default()
            .descend(&ComputedStyle::default(), Arc::new(inherited_custom));
        let style = with_vars(&[("--b", "#0c0000")], &[("color", "var(--b)")]);

        let (computed, next) = style.collapse(&inherited, &ColorAndWidthReparser);

        assert_eq!(computed.color(), Some(&color("#0c0000")));
        assert_eq!(
            next.custom_properties().get("--a").map(String::as_str),
            Some("#0a0000")
        );
        assert_eq!(
            next.custom_properties().get("--b").map(String::as_str),
            Some("#0c0000")
        );
        assert_eq!(
            inherited.custom_properties().get("--b").map(String::as_str),
            Some("#0b0000")
        );
    }

    #[test]
    fn test_no_own_custom_properties_shares_the_inherited_map() {
        let mut inherited_custom = HashMap::new();
        inherited_custom.insert("--a".to_string(), "#0a0000".to_string());
        let inherited = InheritedStyle::default()
            .descend(&ComputedStyle::default(), Arc::new(inherited_custom));

        let (_, next) = with_vars(&[], &[]).collapse(&inherited, &ColorAndWidthReparser);

        assert!(Arc::ptr_eq(
            next.custom_properties(),
            inherited.custom_properties()
        ));
    }

    #[test]
    fn test_pending_vars_apply_after_lengths_and_reach_the_children() {
        let mut lengths = DeclaredLengths::default();
        lengths.set_width(DeclaredLength::Ems(2.0));
        let mut style = with_lengths(ComputedStyle::default(), lengths);
        style.merge_with(&with_vars(
            &[("--w", "5px"), ("--c", "#0d0000")],
            &[("width", "var(--w)"), ("color", "var(--c)")],
        ));

        let (computed, next) = style.collapse(&parent(), &ColorAndWidthReparser);

        assert_eq!(computed.width(), Some(CssLength::Pixels(5.0)));
        assert_eq!(next.color(), Some(&color("#0d0000")));
        assert_eq!(next.font_size(), Some(&FontSize::new(20.0)));
    }
}
