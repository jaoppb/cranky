use super::super::grid_types::{DisplayMode, GridAutoFlow, GridLinePlacement, GridTrack};
use super::super::values::{CssLength, FlexGrow, FlexShrink, Opacity};
use crate::features::layout_engine::domain::{
    AlignItems, BoxMargin, FlexDirection, Gap, JustifyContent, PositionType,
};
use crate::shared::config::domain::{BorderRadius, BorderSize, FontFamily, FontSize};
use crate::shared::primitives::color::DrawingColor;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ComputedStyle {
    pub(crate) background: Option<DrawingColor>,
    pub(crate) color: Option<DrawingColor>,
    pub(crate) accent_color: Option<DrawingColor>,
    pub(crate) font_family: Option<FontFamily>,
    pub(crate) font_size: Option<FontSize>,
    pub(crate) border_size: Option<BorderSize>,
    pub(crate) border_color: Option<DrawingColor>,
    pub(crate) border_radius: Option<BorderRadius>,
    pub(crate) width: Option<CssLength>,
    pub(crate) height: Option<CssLength>,
    pub(crate) min_width: Option<CssLength>,
    pub(crate) min_height: Option<CssLength>,
    pub(crate) max_width: Option<CssLength>,
    pub(crate) max_height: Option<CssLength>,
    pub(crate) padding: Option<BoxMargin>,
    pub(crate) margin: Option<BoxMargin>,
    pub(crate) gap: Option<Gap>,
    pub(crate) column_gap: Option<Gap>,
    pub(crate) row_gap: Option<Gap>,
    pub(crate) display: Option<DisplayMode>,
    pub(crate) flex_direction: Option<FlexDirection>,
    pub(crate) justify_content: Option<JustifyContent>,
    pub(crate) align_items: Option<AlignItems>,
    pub(crate) justify_items: Option<AlignItems>,
    pub(crate) justify_self: Option<AlignItems>,
    pub(crate) align_content: Option<JustifyContent>,
    pub(crate) grid_template_columns: Option<Vec<GridTrack>>,
    pub(crate) grid_template_rows: Option<Vec<GridTrack>>,
    pub(crate) grid_auto_columns: Option<Vec<GridTrack>>,
    pub(crate) grid_auto_rows: Option<Vec<GridTrack>>,
    pub(crate) grid_auto_flow: Option<GridAutoFlow>,
    pub(crate) grid_column: Option<GridLinePlacement>,
    pub(crate) grid_row: Option<GridLinePlacement>,
    pub(crate) position: Option<PositionType>,
    pub(crate) opacity: Option<Opacity>,
    pub(crate) flex_grow: Option<FlexGrow>,
    pub(crate) flex_shrink: Option<FlexShrink>,
    pub(crate) flex_basis: Option<CssLength>,
    pub(crate) align_self: Option<AlignItems>,
}

impl ComputedStyle {
    #[must_use]
    pub fn default_for_flex() -> Self {
        Self {
            display: Some(DisplayMode::Flex),
            flex_direction: Some(FlexDirection::Row),
            align_items: Some(AlignItems::Start),
            ..Default::default()
        }
    }

    #[must_use]
    pub fn default_for_grid() -> Self {
        Self {
            display: Some(DisplayMode::Grid),
            grid_auto_flow: Some(GridAutoFlow::Row),
            ..Default::default()
        }
    }

    pub fn merge_with(&mut self, other: &Self) {
        macro_rules! merge_fields {
            ($($field:ident),* $(,)?) => {
                $(
                    if other.$field.is_some() {
                        self.$field.clone_from(&other.$field);
                    }
                )*
            };
        }

        merge_fields!(
            background,
            color,
            accent_color,
            font_family,
            font_size,
            border_size,
            border_color,
            border_radius,
            width,
            height,
            min_width,
            min_height,
            max_width,
            max_height,
            padding,
            margin,
            gap,
            column_gap,
            row_gap,
            display,
            flex_direction,
            justify_content,
            align_items,
            justify_items,
            justify_self,
            align_content,
            grid_template_columns,
            grid_template_rows,
            grid_auto_columns,
            grid_auto_rows,
            grid_auto_flow,
            grid_column,
            grid_row,
            position,
            opacity,
            flex_grow,
            flex_shrink,
            flex_basis,
            align_self,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_computed_style_merge() {
        let mut style1 = ComputedStyle::default();
        style1.set_font_size(FontSize::new(12.0));

        let mut style2 = ComputedStyle::default();
        style2.set_font_size(FontSize::new(16.0));
        style2.set_border_radius(BorderRadius::new(4.0));
        style2.set_opacity(Opacity::new(0.8).unwrap());
        style2.set_flex_grow(FlexGrow::new(1.0).unwrap());
        style2.set_flex_shrink(FlexShrink::new(0.0).unwrap());

        style1.merge_with(&style2);
        assert!((style1.font_size().unwrap().value() - 16.0).abs() < f32::EPSILON);
        assert!((style1.border_radius().unwrap().value() - 4.0).abs() < f32::EPSILON);
        assert!((style1.opacity().unwrap().value() - 0.8).abs() < f32::EPSILON);
        assert!((style1.flex_grow().unwrap().value() - 1.0).abs() < f32::EPSILON);
        assert!((style1.flex_shrink().unwrap().value()).abs() < f32::EPSILON);
    }
}
