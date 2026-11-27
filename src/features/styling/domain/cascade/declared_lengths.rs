//! The cascaded, still unresolved lengths of the twenty length-bearing properties.
use super::declared_length::DeclaredLength;
use super::value::Cascade;

/// The twenty length-bearing properties that can carry `em`/`rem`.
///
/// Besides `font-size`, resolved separately since other lengths need its
/// result as their `em` base — tracked as `Cascade<DeclaredLength>` instead
/// of living directly on `ComputedStyle`, exactly like the four inheritable
/// fields in `DeclaredStyle`, and for the same reason: resolving `em`/`rem`
/// needs the element's own font size, which only exists once the cascade
/// has been folded for this element.
///
/// `apply` — turning these into concrete pixels on a `ComputedStyle` — lives
/// in `declared_lengths_apply.rs`, kept separate to stay under the 200 LOC
/// production file limit.
#[derive(Debug, Clone, Default)]
pub(crate) struct DeclaredLengths {
    pub(super) width: Cascade<DeclaredLength>,
    pub(super) height: Cascade<DeclaredLength>,
    pub(super) min_width: Cascade<DeclaredLength>,
    pub(super) min_height: Cascade<DeclaredLength>,
    pub(super) max_width: Cascade<DeclaredLength>,
    pub(super) max_height: Cascade<DeclaredLength>,
    pub(super) flex_basis: Cascade<DeclaredLength>,
    pub(super) border_size: Cascade<DeclaredLength>,
    pub(super) border_radius: Cascade<DeclaredLength>,
    pub(super) gap: Cascade<DeclaredLength>,
    pub(super) column_gap: Cascade<DeclaredLength>,
    pub(super) row_gap: Cascade<DeclaredLength>,
    pub(super) padding_top: Cascade<DeclaredLength>,
    pub(super) padding_right: Cascade<DeclaredLength>,
    pub(super) padding_bottom: Cascade<DeclaredLength>,
    pub(super) padding_left: Cascade<DeclaredLength>,
    pub(super) margin_top: Cascade<DeclaredLength>,
    pub(super) margin_right: Cascade<DeclaredLength>,
    pub(super) margin_bottom: Cascade<DeclaredLength>,
    pub(super) margin_left: Cascade<DeclaredLength>,
}

macro_rules! setter {
    ($name:ident, $setter:ident) => {
        pub(crate) const fn $setter(&mut self, value: DeclaredLength) {
            self.$name = Cascade::Value(value);
        }
    };
}

impl DeclaredLengths {
    setter!(width, set_width);
    setter!(height, set_height);
    setter!(min_width, set_min_width);
    setter!(min_height, set_min_height);
    setter!(max_width, set_max_width);
    setter!(max_height, set_max_height);
    setter!(flex_basis, set_flex_basis);
    setter!(border_size, set_border_size);
    setter!(border_radius, set_border_radius);
    setter!(gap, set_gap);
    setter!(column_gap, set_column_gap);
    setter!(row_gap, set_row_gap);
    setter!(padding_top, set_padding_top);
    setter!(padding_right, set_padding_right);
    setter!(padding_bottom, set_padding_bottom);
    setter!(padding_left, set_padding_left);
    setter!(margin_top, set_margin_top);
    setter!(margin_right, set_margin_right);
    setter!(margin_bottom, set_margin_bottom);
    setter!(margin_left, set_margin_left);

    pub(crate) fn merge_with(&mut self, other: &Self) {
        macro_rules! merge_fields {
            ($($field:ident),* $(,)?) => {
                $(
                    if !matches!(other.$field, Cascade::NotDeclared) {
                        self.$field = other.$field.clone();
                    }
                )*
            };
        }
        merge_fields!(
            width,
            height,
            min_width,
            min_height,
            max_width,
            max_height,
            flex_basis,
            border_size,
            border_radius,
            gap,
            column_gap,
            row_gap,
            padding_top,
            padding_right,
            padding_bottom,
            padding_left,
            margin_top,
            margin_right,
            margin_bottom,
            margin_left,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::super::declared_lengths_apply::FontSizeBases;
    use super::*;
    use crate::shared::config::domain::FontSize;

    #[test]
    fn test_merge_prefers_declared_over_not_declared() {
        let mut base = DeclaredLengths::default();
        base.set_width(DeclaredLength::Pixels(10.0));
        let mut later = DeclaredLengths::default();
        later.set_height(DeclaredLength::Pixels(20.0));

        base.merge_with(&later);

        let font_size = FontSize::new(14.0);
        let bases = FontSizeBases::new(font_size, font_size);
        assert_eq!(
            bases.resolve(base.width),
            Some(crate::features::styling::domain::CssLength::Pixels(10.0))
        );
        assert_eq!(
            bases.resolve(base.height),
            Some(crate::features::styling::domain::CssLength::Pixels(20.0))
        );
    }

    /// The twenty fields in declaration order, to compare two `DeclaredLengths` whole.
    fn fields(lengths: &DeclaredLengths) -> [Cascade<DeclaredLength>; 20] {
        [
            lengths.width,
            lengths.height,
            lengths.min_width,
            lengths.min_height,
            lengths.max_width,
            lengths.max_height,
            lengths.flex_basis,
            lengths.border_size,
            lengths.border_radius,
            lengths.gap,
            lengths.column_gap,
            lengths.row_gap,
            lengths.padding_top,
            lengths.padding_right,
            lengths.padding_bottom,
            lengths.padding_left,
            lengths.margin_top,
            lengths.margin_right,
            lengths.margin_bottom,
            lengths.margin_left,
        ]
    }

    /// The twenty setters, in declaration order.
    fn setters() -> [fn(&mut DeclaredLengths, DeclaredLength); 20] {
        [
            DeclaredLengths::set_width,
            DeclaredLengths::set_height,
            DeclaredLengths::set_min_width,
            DeclaredLengths::set_min_height,
            DeclaredLengths::set_max_width,
            DeclaredLengths::set_max_height,
            DeclaredLengths::set_flex_basis,
            DeclaredLengths::set_border_size,
            DeclaredLengths::set_border_radius,
            DeclaredLengths::set_gap,
            DeclaredLengths::set_column_gap,
            DeclaredLengths::set_row_gap,
            DeclaredLengths::set_padding_top,
            DeclaredLengths::set_padding_right,
            DeclaredLengths::set_padding_bottom,
            DeclaredLengths::set_padding_left,
            DeclaredLengths::set_margin_top,
            DeclaredLengths::set_margin_right,
            DeclaredLengths::set_margin_bottom,
            DeclaredLengths::set_margin_left,
        ]
    }

    /// Every field declared through its own setter, each to `seed` plus its position, so no two
    /// fields hold the same length.
    fn every_length(seed: f32) -> DeclaredLengths {
        let mut lengths = DeclaredLengths::default();
        let mut value = seed;
        for setter in setters() {
            value += 1.0;
            setter(&mut lengths, DeclaredLength::Pixels(value));
        }
        lengths
    }

    #[test]
    fn test_merge_carries_every_declared_length() {
        let later = every_length(0.0);
        let mut base = DeclaredLengths::default();

        base.merge_with(&later);

        assert_eq!(fields(&base), fields(&later));
    }

    #[test]
    fn test_merge_keeps_every_length_the_later_rule_does_not_declare() {
        let mut base = every_length(0.0);

        base.merge_with(&DeclaredLengths::default());

        assert_eq!(fields(&base), fields(&every_length(0.0)));
    }

    #[test]
    fn test_merge_lets_the_later_rule_win_on_every_length() {
        let later = every_length(100.0);
        let mut base = every_length(0.0);

        base.merge_with(&later);

        assert_eq!(fields(&base), fields(&later));
    }

    #[test]
    fn test_merge_takes_a_later_css_wide_keyword_over_a_declared_length() {
        let mut base = every_length(0.0);
        let later = DeclaredLengths {
            width: Cascade::Initial,
            margin_left: Cascade::Inherit,
            ..DeclaredLengths::default()
        };

        base.merge_with(&later);

        assert_eq!(base.width, Cascade::Initial);
        assert_eq!(base.margin_left, Cascade::Inherit);
        assert_eq!(base.height, Cascade::Value(DeclaredLength::Pixels(2.0)));
    }
}
