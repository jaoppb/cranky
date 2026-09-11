use super::declared_length::DeclaredLength;
use super::value::Cascade;
use crate::shared::config::domain::FontSize;

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
pub struct DeclaredLengths {
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
        pub const fn $setter(&mut self, value: DeclaredLength) {
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

    pub fn merge_with(&mut self, other: &Self) {
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

pub(super) fn resolve(
    cascade: Cascade<DeclaredLength>,
    font_size: FontSize,
    root_font_size: FontSize,
) -> Option<crate::features::styling::domain::CssLength> {
    match cascade {
        Cascade::Value(v) => Some(v.resolve(font_size, root_font_size)),
        Cascade::NotDeclared | Cascade::Inherit | Cascade::Initial => None,
    }
}

pub(super) fn resolve_scalar(
    cascade: Cascade<DeclaredLength>,
    font_size: FontSize,
    root_font_size: FontSize,
) -> Option<f32> {
    match cascade {
        Cascade::Value(v) => Some(v.resolve_scalar(font_size, root_font_size)),
        Cascade::NotDeclared | Cascade::Inherit | Cascade::Initial => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge_prefers_declared_over_not_declared() {
        let mut base = DeclaredLengths::default();
        base.set_width(DeclaredLength::Px(10.0));
        let mut later = DeclaredLengths::default();
        later.set_height(DeclaredLength::Px(20.0));

        base.merge_with(&later);

        let fs = FontSize::new(14.0);
        assert_eq!(
            resolve(base.width, fs, fs),
            Some(crate::features::styling::domain::CssLength::Px(10.0))
        );
        assert_eq!(
            resolve(base.height, fs, fs),
            Some(crate::features::styling::domain::CssLength::Px(20.0))
        );
    }
}
