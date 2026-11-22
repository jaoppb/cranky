//! Resolves the declared lengths against the element's font size and writes them as pixels
//! onto a computed style.
use super::super::CssLength;
use super::super::computed_style::ComputedStyle;
use super::declared_length::DeclaredLength;
use super::declared_lengths::DeclaredLengths;
use super::value::Cascade;
use crate::features::layout_engine::domain::{BoxMargin, Gap};
use crate::shared::config::domain::{BorderRadius, BorderSize, FontSize};

/// The two font sizes a font-relative length resolves against: the element's own (the `em`
/// base) and the root element's (the `rem` base).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct FontSizeBases {
    font_size: FontSize,
    root_font_size: FontSize,
}

impl FontSizeBases {
    /// Resolves `em` against `font_size` and `rem` against `root_font_size`.
    #[must_use]
    pub(crate) const fn new(font_size: FontSize, root_font_size: FontSize) -> Self {
        Self {
            font_size,
            root_font_size,
        }
    }

    /// `cascade`'s length in pixels or percent, or `None` when it carries no value of its own.
    #[must_use]
    pub(super) fn resolve(self, cascade: Cascade<DeclaredLength>) -> Option<CssLength> {
        match cascade {
            Cascade::Value(declared) => Some(declared.resolve(self.font_size, self.root_font_size)),
            Cascade::NotDeclared | Cascade::Inherit | Cascade::Initial => None,
        }
    }

    /// `cascade`'s length in pixels, or `None` when it carries no value of its own.
    #[must_use]
    pub(super) fn resolve_scalar(self, cascade: Cascade<DeclaredLength>) -> Option<f32> {
        match cascade {
            Cascade::Value(declared) => {
                Some(declared.resolve_scalar(self.font_size, self.root_font_size))
            }
            Cascade::NotDeclared | Cascade::Inherit | Cascade::Initial => None,
        }
    }
}

impl DeclaredLengths {
    /// Resolves every declared length against `bases`: this element's own,
    /// already-resolved font size (the `em` base for everything except
    /// `font-size` itself) and the root's (the `rem` base), writing
    /// results into `computed`. Properties left `NotDeclared`/`Initial`
    /// here simply leave `computed`'s existing value alone — it was already
    /// set correctly by the ordinary (non-relative) property pipeline.
    pub(crate) fn apply(&self, computed: &mut ComputedStyle, bases: FontSizeBases) {
        self.apply_css_lengths(computed, bases);
        self.apply_scalars(computed, bases);
        self.apply_padding(computed, bases);
        self.apply_margin(computed, bases);
    }

    fn apply_css_lengths(&self, computed: &mut ComputedStyle, bases: FontSizeBases) {
        if let Some(length) = bases.resolve(self.width) {
            computed.set_width(length);
        }
        if let Some(length) = bases.resolve(self.height) {
            computed.set_height(length);
        }
        if let Some(length) = bases.resolve(self.min_width) {
            computed.set_min_width(length);
        }
        if let Some(length) = bases.resolve(self.min_height) {
            computed.set_min_height(length);
        }
        if let Some(length) = bases.resolve(self.max_width) {
            computed.set_max_width(length);
        }
        if let Some(length) = bases.resolve(self.max_height) {
            computed.set_max_height(length);
        }
        if let Some(length) = bases.resolve(self.flex_basis) {
            computed.set_flex_basis(length);
        }
    }

    fn apply_scalars(&self, computed: &mut ComputedStyle, bases: FontSizeBases) {
        if let Some(pixels) = bases.resolve_scalar(self.border_size) {
            computed.set_border_size(BorderSize::new(pixels));
        }
        if let Some(pixels) = bases.resolve_scalar(self.border_radius) {
            computed.set_border_radius(BorderRadius::new(pixels));
        }
        if let Some(pixels) = bases.resolve_scalar(self.gap) {
            computed.set_gap(Gap::new(f64::from(pixels)));
        }
        if let Some(pixels) = bases.resolve_scalar(self.column_gap) {
            computed.set_column_gap(Gap::new(f64::from(pixels)));
        }
        if let Some(pixels) = bases.resolve_scalar(self.row_gap) {
            computed.set_row_gap(Gap::new(f64::from(pixels)));
        }
    }

    fn apply_padding(&self, computed: &mut ComputedStyle, bases: FontSizeBases) {
        if matches!(self.padding_top, Cascade::NotDeclared)
            && matches!(self.padding_right, Cascade::NotDeclared)
            && matches!(self.padding_bottom, Cascade::NotDeclared)
            && matches!(self.padding_left, Cascade::NotDeclared)
        {
            return;
        }
        let current = computed.padding().cloned().unwrap_or_default();
        let top = bases
            .resolve_scalar(self.padding_top)
            .map_or_else(|| current.top(), f64::from);
        let right = bases
            .resolve_scalar(self.padding_right)
            .map_or_else(|| current.right(), f64::from);
        let bottom = bases
            .resolve_scalar(self.padding_bottom)
            .map_or_else(|| current.bottom(), f64::from);
        let left = bases
            .resolve_scalar(self.padding_left)
            .map_or_else(|| current.left(), f64::from);
        computed.set_padding(BoxMargin::new(top, bottom, left, right));
    }

    fn apply_margin(&self, computed: &mut ComputedStyle, bases: FontSizeBases) {
        if matches!(self.margin_top, Cascade::NotDeclared)
            && matches!(self.margin_right, Cascade::NotDeclared)
            && matches!(self.margin_bottom, Cascade::NotDeclared)
            && matches!(self.margin_left, Cascade::NotDeclared)
        {
            return;
        }
        let current = computed.margin().cloned().unwrap_or_default();
        let top = bases
            .resolve_scalar(self.margin_top)
            .map_or_else(|| current.top(), f64::from);
        let right = bases
            .resolve_scalar(self.margin_right)
            .map_or_else(|| current.right(), f64::from);
        let bottom = bases
            .resolve_scalar(self.margin_bottom)
            .map_or_else(|| current.bottom(), f64::from);
        let left = bases
            .resolve_scalar(self.margin_left)
            .map_or_else(|| current.left(), f64::from);
        computed.set_margin(BoxMargin::new(top, bottom, left, right));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::styling::domain::DeclaredLength;

    #[test]
    fn test_apply_em_padding_uses_font_size_as_base() {
        let mut lengths = DeclaredLengths::default();
        lengths.set_padding_top(DeclaredLength::Ems(2.0));

        let mut computed = ComputedStyle::default();
        lengths.apply(
            &mut computed,
            FontSizeBases::new(FontSize::new(8.0), FontSize::new(14.0)),
        );

        assert_eq!(computed.padding().map(BoxMargin::top), Some(16.0));
    }

    #[test]
    fn test_apply_leaves_unset_targets_untouched() {
        let lengths = DeclaredLengths::default();
        let mut computed = ComputedStyle::default();
        computed.set_border_size(BorderSize::new(3.0));

        lengths.apply(
            &mut computed,
            FontSizeBases::new(FontSize::new(14.0), FontSize::new(14.0)),
        );

        assert_eq!(computed.border_size().map(BorderSize::value), Some(3.0));
    }

    const fn px(value: f32) -> crate::features::styling::domain::CssLength {
        crate::features::styling::domain::CssLength::Pixels(value)
    }

    #[test]
    fn test_apply_resolves_css_lengths_against_em_and_rem_bases() {
        let mut lengths = DeclaredLengths::default();
        lengths.set_width(DeclaredLength::Ems(2.0));
        lengths.set_height(DeclaredLength::Rem(2.0));
        lengths.set_min_width(DeclaredLength::Ems(1.0));
        lengths.set_min_height(DeclaredLength::Rem(1.0));
        lengths.set_max_width(DeclaredLength::Ems(3.0));
        lengths.set_max_height(DeclaredLength::Rem(3.0));
        lengths.set_flex_basis(DeclaredLength::Ems(0.5));

        let mut computed = ComputedStyle::default();
        lengths.apply(
            &mut computed,
            FontSizeBases::new(FontSize::new(8.0), FontSize::new(14.0)),
        );

        assert_eq!(computed.width(), Some(px(16.0)));
        assert_eq!(computed.height(), Some(px(28.0)));
        assert_eq!(computed.min_width(), Some(px(8.0)));
        assert_eq!(computed.min_height(), Some(px(14.0)));
        assert_eq!(computed.max_width(), Some(px(24.0)));
        assert_eq!(computed.max_height(), Some(px(42.0)));
        assert_eq!(computed.flex_basis(), Some(px(4.0)));
    }

    #[test]
    fn test_apply_resolves_scalars_against_em_and_rem_bases() {
        let mut lengths = DeclaredLengths::default();
        lengths.set_border_size(DeclaredLength::Ems(0.25));
        lengths.set_border_radius(DeclaredLength::Rem(0.5));
        lengths.set_gap(DeclaredLength::Ems(1.5));
        lengths.set_column_gap(DeclaredLength::Rem(2.0));
        lengths.set_row_gap(DeclaredLength::Ems(3.0));

        let mut computed = ComputedStyle::default();
        lengths.apply(
            &mut computed,
            FontSizeBases::new(FontSize::new(8.0), FontSize::new(14.0)),
        );

        assert_eq!(computed.border_size().map(BorderSize::value), Some(2.0));
        assert_eq!(computed.border_radius().map(BorderRadius::value), Some(7.0));
        assert_eq!(computed.gap().map(Gap::value), Some(12.0));
        assert_eq!(computed.column_gap().map(Gap::value), Some(28.0));
        assert_eq!(computed.row_gap().map(Gap::value), Some(24.0));
    }

    #[test]
    fn test_apply_padding_resolves_declared_sides_and_keeps_the_others() {
        let mut lengths = DeclaredLengths::default();
        lengths.set_padding_right(DeclaredLength::Rem(1.0));
        lengths.set_padding_bottom(DeclaredLength::Ems(0.5));

        let mut computed = ComputedStyle::default();
        computed.set_padding(BoxMargin::new(1.0, 2.0, 3.0, 4.0));
        lengths.apply(
            &mut computed,
            FontSizeBases::new(FontSize::new(8.0), FontSize::new(14.0)),
        );

        assert_eq!(
            computed.padding(),
            Some(&BoxMargin::new(1.0, 4.0, 3.0, 14.0))
        );
    }

    #[test]
    fn test_apply_margin_resolves_declared_sides_and_keeps_the_others() {
        let mut lengths = DeclaredLengths::default();
        lengths.set_margin_top(DeclaredLength::Ems(2.0));
        lengths.set_margin_left(DeclaredLength::Rem(0.5));

        let mut computed = ComputedStyle::default();
        computed.set_margin(BoxMargin::new(5.0, 6.0, 1.0, 9.0));
        lengths.apply(
            &mut computed,
            FontSizeBases::new(FontSize::new(8.0), FontSize::new(14.0)),
        );

        assert_eq!(
            computed.margin(),
            Some(&BoxMargin::new(16.0, 6.0, 7.0, 9.0))
        );

        let mut bare = ComputedStyle::default();
        lengths.apply(
            &mut bare,
            FontSizeBases::new(FontSize::new(8.0), FontSize::new(14.0)),
        );
        assert_eq!(bare.margin(), Some(&BoxMargin::new(16.0, 0.0, 7.0, 0.0)));
    }
}
