use super::super::computed_style::ComputedStyle;
use super::declared_lengths::{DeclaredLengths, resolve, resolve_scalar};
use super::value::Cascade;
use crate::features::layout_engine::domain::{BoxMargin, Gap};
use crate::shared::config::domain::{BorderRadius, BorderSize, FontSize};

impl DeclaredLengths {
    /// Resolves every declared length against `font_size` (this element's
    /// own, already-resolved size — the `em` base for everything except
    /// `font-size` itself) and `root_font_size` (the `rem` base), writing
    /// results into `computed`. Properties left `NotDeclared`/`Initial`
    /// here simply leave `computed`'s existing value alone — it was already
    /// set correctly by the ordinary (non-relative) property pipeline.
    pub fn apply(
        &self,
        computed: &mut ComputedStyle,
        font_size: FontSize,
        root_font_size: FontSize,
    ) {
        self.apply_css_lengths(computed, font_size, root_font_size);
        self.apply_scalars(computed, font_size, root_font_size);
        self.apply_padding(computed, font_size, root_font_size);
        self.apply_margin(computed, font_size, root_font_size);
    }

    fn apply_css_lengths(&self, computed: &mut ComputedStyle, fs: FontSize, root: FontSize) {
        if let Some(v) = resolve(self.width, fs, root) {
            computed.set_width(v);
        }
        if let Some(v) = resolve(self.height, fs, root) {
            computed.set_height(v);
        }
        if let Some(v) = resolve(self.min_width, fs, root) {
            computed.set_min_width(v);
        }
        if let Some(v) = resolve(self.min_height, fs, root) {
            computed.set_min_height(v);
        }
        if let Some(v) = resolve(self.max_width, fs, root) {
            computed.set_max_width(v);
        }
        if let Some(v) = resolve(self.max_height, fs, root) {
            computed.set_max_height(v);
        }
        if let Some(v) = resolve(self.flex_basis, fs, root) {
            computed.set_flex_basis(v);
        }
    }

    fn apply_scalars(&self, computed: &mut ComputedStyle, fs: FontSize, root: FontSize) {
        if let Some(v) = resolve_scalar(self.border_size, fs, root) {
            computed.set_border_size(BorderSize::new(v));
        }
        if let Some(v) = resolve_scalar(self.border_radius, fs, root) {
            computed.set_border_radius(BorderRadius::new(v));
        }
        if let Some(v) = resolve_scalar(self.gap, fs, root) {
            computed.set_gap(Gap::new(f64::from(v)));
        }
        if let Some(v) = resolve_scalar(self.column_gap, fs, root) {
            computed.set_column_gap(Gap::new(f64::from(v)));
        }
        if let Some(v) = resolve_scalar(self.row_gap, fs, root) {
            computed.set_row_gap(Gap::new(f64::from(v)));
        }
    }

    fn apply_padding(&self, computed: &mut ComputedStyle, fs: FontSize, root: FontSize) {
        if matches!(self.padding_top, Cascade::NotDeclared)
            && matches!(self.padding_right, Cascade::NotDeclared)
            && matches!(self.padding_bottom, Cascade::NotDeclared)
            && matches!(self.padding_left, Cascade::NotDeclared)
        {
            return;
        }
        let current = computed.padding().cloned().unwrap_or_default();
        let top =
            resolve_scalar(self.padding_top, fs, root).map_or_else(|| current.top(), f64::from);
        let right =
            resolve_scalar(self.padding_right, fs, root).map_or_else(|| current.right(), f64::from);
        let bottom = resolve_scalar(self.padding_bottom, fs, root)
            .map_or_else(|| current.bottom(), f64::from);
        let left =
            resolve_scalar(self.padding_left, fs, root).map_or_else(|| current.left(), f64::from);
        computed.set_padding(BoxMargin::new(top, bottom, left, right));
    }

    fn apply_margin(&self, computed: &mut ComputedStyle, fs: FontSize, root: FontSize) {
        if matches!(self.margin_top, Cascade::NotDeclared)
            && matches!(self.margin_right, Cascade::NotDeclared)
            && matches!(self.margin_bottom, Cascade::NotDeclared)
            && matches!(self.margin_left, Cascade::NotDeclared)
        {
            return;
        }
        let current = computed.margin().cloned().unwrap_or_default();
        let top =
            resolve_scalar(self.margin_top, fs, root).map_or_else(|| current.top(), f64::from);
        let right =
            resolve_scalar(self.margin_right, fs, root).map_or_else(|| current.right(), f64::from);
        let bottom = resolve_scalar(self.margin_bottom, fs, root)
            .map_or_else(|| current.bottom(), f64::from);
        let left =
            resolve_scalar(self.margin_left, fs, root).map_or_else(|| current.left(), f64::from);
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
        lengths.set_padding_top(DeclaredLength::Em(2.0));

        let mut computed = ComputedStyle::default();
        lengths.apply(&mut computed, FontSize::new(8.0), FontSize::new(14.0));

        assert_eq!(computed.padding().map(BoxMargin::top), Some(16.0));
    }

    #[test]
    fn test_apply_leaves_unset_targets_untouched() {
        let lengths = DeclaredLengths::default();
        let mut computed = ComputedStyle::default();
        computed.set_border_size(BorderSize::new(3.0));

        lengths.apply(&mut computed, FontSize::new(14.0), FontSize::new(14.0));

        assert_eq!(computed.border_size().map(|b| b.value()), Some(3.0));
    }
}
