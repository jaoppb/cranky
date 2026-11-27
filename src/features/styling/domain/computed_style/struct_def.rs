//! The [`ComputedStyle`] struct, its flex and grid defaults, and the merge used by the cascade.
use super::super::grid_types::{DisplayMode, GridAutoFlow, GridLinePlacement, GridTrack};
use super::super::values::{CssLength, FlexGrow, FlexShrink, Opacity};
use crate::features::layout_engine::domain::{
    AlignItems, BoxMargin, FlexDirection, Gap, JustifyContent, PositionType,
};
use crate::shared::config::domain::{BorderRadius, BorderSize, FontFamily, FontSize};
use crate::shared::primitives::TextAlign;
use crate::shared::primitives::color::DrawingColor;

/// Every property the renderer understands, each `None` unless a rule, inheritance or the
/// flex/grid defaults set it; the layout and paint phases fall back to their own defaults for
/// `None`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ComputedStyle {
    pub(crate) background: Option<DrawingColor>,
    pub(crate) color: Option<DrawingColor>,
    pub(crate) accent_color: Option<DrawingColor>,
    pub(crate) font_family: Option<FontFamily>,
    pub(crate) font_size: Option<FontSize>,
    pub(crate) text_align: Option<TextAlign>,
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
    pub(crate) fn default_for_flex() -> Self {
        Self {
            display: Some(DisplayMode::Flex),
            flex_direction: Some(FlexDirection::Row),
            align_items: Some(AlignItems::Stretch),
            ..Default::default()
        }
    }

    #[must_use]
    pub(crate) fn default_for_grid() -> Self {
        Self {
            display: Some(DisplayMode::Grid),
            grid_auto_flow: Some(GridAutoFlow::Row),
            ..Default::default()
        }
    }

    /// Overlays `other` on this style: every property `other` sets replaces this one, every
    /// property it leaves `None` keeps this one.
    pub(crate) fn merge_with(&mut self, other: &Self) {
        self.merge_text_and_border(other);
        self.merge_sizes_and_spacing(other);
        self.merge_container_layout(other);
        self.merge_position_opacity_and_flex_item(other);
    }

    /// Merges the colors, the font and text alignment, and the border.
    fn merge_text_and_border(&mut self, other: &Self) {
        take_if_set(&mut self.background, other.background.as_ref());
        take_if_set(&mut self.color, other.color.as_ref());
        take_if_set(&mut self.accent_color, other.accent_color.as_ref());
        take_if_set(&mut self.font_family, other.font_family.as_ref());
        take_if_set(&mut self.font_size, other.font_size.as_ref());
        take_if_set(&mut self.text_align, other.text_align.as_ref());
        take_if_set(&mut self.border_size, other.border_size.as_ref());
        take_if_set(&mut self.border_color, other.border_color.as_ref());
        take_if_set(&mut self.border_radius, other.border_radius.as_ref());
    }

    /// Merges the width and height with their bounds, the padding and margin, and the gaps.
    fn merge_sizes_and_spacing(&mut self, other: &Self) {
        take_if_set(&mut self.width, other.width.as_ref());
        take_if_set(&mut self.height, other.height.as_ref());
        take_if_set(&mut self.min_width, other.min_width.as_ref());
        take_if_set(&mut self.min_height, other.min_height.as_ref());
        take_if_set(&mut self.max_width, other.max_width.as_ref());
        take_if_set(&mut self.max_height, other.max_height.as_ref());
        take_if_set(&mut self.padding, other.padding.as_ref());
        take_if_set(&mut self.margin, other.margin.as_ref());
        take_if_set(&mut self.gap, other.gap.as_ref());
        take_if_set(&mut self.column_gap, other.column_gap.as_ref());
        take_if_set(&mut self.row_gap, other.row_gap.as_ref());
    }

    /// Merges how the element lays out its children: display, flex direction, alignment, grid.
    fn merge_container_layout(&mut self, other: &Self) {
        take_if_set(&mut self.display, other.display.as_ref());
        take_if_set(&mut self.flex_direction, other.flex_direction.as_ref());
        take_if_set(&mut self.justify_content, other.justify_content.as_ref());
        take_if_set(&mut self.align_items, other.align_items.as_ref());
        take_if_set(&mut self.justify_items, other.justify_items.as_ref());
        take_if_set(&mut self.justify_self, other.justify_self.as_ref());
        take_if_set(&mut self.align_content, other.align_content.as_ref());
        take_if_set(
            &mut self.grid_template_columns,
            other.grid_template_columns.as_ref(),
        );
        take_if_set(
            &mut self.grid_template_rows,
            other.grid_template_rows.as_ref(),
        );
        take_if_set(
            &mut self.grid_auto_columns,
            other.grid_auto_columns.as_ref(),
        );
        take_if_set(&mut self.grid_auto_rows, other.grid_auto_rows.as_ref());
        take_if_set(&mut self.grid_auto_flow, other.grid_auto_flow.as_ref());
        take_if_set(&mut self.grid_column, other.grid_column.as_ref());
        take_if_set(&mut self.grid_row, other.grid_row.as_ref());
    }

    /// Merges the positioning, the opacity, and how the element sizes and aligns as a flex item.
    fn merge_position_opacity_and_flex_item(&mut self, other: &Self) {
        take_if_set(&mut self.position, other.position.as_ref());
        take_if_set(&mut self.opacity, other.opacity.as_ref());
        take_if_set(&mut self.flex_grow, other.flex_grow.as_ref());
        take_if_set(&mut self.flex_shrink, other.flex_shrink.as_ref());
        take_if_set(&mut self.flex_basis, other.flex_basis.as_ref());
        take_if_set(&mut self.align_self, other.align_self.as_ref());
    }
}

/// Replaces `target` with `later` when `later` is set, so a later rule's declaration wins and a
/// property it leaves out keeps the earlier value.
fn take_if_set<T: Clone>(target: &mut Option<T>, later: Option<&T>) {
    if let Some(value) = later {
        *target = Some(value.clone());
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

    use super::super::super::grid_types::GridPlacement;
    use crate::shared::config::domain::AxisMargins;

    /// The numbers, colors and enum variants one fixture is built from: a different seed changes
    /// every number and picks a different variant of every enum.
    #[derive(Clone, Copy)]
    struct Seed(u8);

    impl Seed {
        fn flip(self) -> usize {
            usize::from(self.0 != 0)
        }

        fn number(self, index: f32) -> f32 {
            f32::from(self.0) + index
        }

        fn color(self, index: u8) -> DrawingColor {
            DrawingColor::parse(&format!("#{:02x}{index:02x}00", self.0)).unwrap()
        }

        fn pixels(self, index: f32) -> CssLength {
            CssLength::Pixels(self.number(index))
        }

        fn margin(self, index: f32) -> BoxMargin {
            let vertical = AxisMargins::new(f64::from(self.number(index)), 0.0);
            BoxMargin::new(vertical, AxisMargins::new(0.0, 0.0))
        }

        fn gap(self, index: f32) -> Gap {
            Gap::new(f64::from(self.number(index)))
        }

        fn tracks(self, index: f32) -> Vec<GridTrack> {
            vec![GridTrack::Pixels(self.number(index))]
        }

        fn line(self, index: i16) -> GridLinePlacement {
            let span = GridPlacement::Span(u16::from(self.0));
            GridLinePlacement::new(GridPlacement::Line(index), span)
        }

        /// Four distinct alignments, rotated by the seed.
        fn aligns(self) -> Vec<AlignItems> {
            let all = [
                AlignItems::Start,
                AlignItems::End,
                AlignItems::Center,
                AlignItems::Stretch,
            ];
            all.into_iter().cycle().skip(self.flip()).take(4).collect()
        }
    }

    /// A style with all forty properties set, each to a value no other property of the same
    /// type shares, so a dropped or crossed field in the merge shows up as a difference.
    fn every_property(seed: u8) -> ComputedStyle {
        let seed = Seed(seed);
        let mut style = ComputedStyle::default();
        set_text_and_border(&mut style, seed);
        set_sizes_and_spacing(&mut style, seed);
        set_container_layout(&mut style, seed);
        set_grid(&mut style, seed);
        set_item(&mut style, seed);
        style
    }

    fn set_text_and_border(style: &mut ComputedStyle, seed: Seed) {
        style.background = Some(seed.color(1));
        style.color = Some(seed.color(2));
        style.accent_color = Some(seed.color(3));
        style.font_family = Some(FontFamily::new(format!("family {}", seed.0)));
        style.font_size = Some(FontSize::new(seed.number(4.0)));
        style.text_align = Some([TextAlign::Left, TextAlign::Center][seed.flip()]);
        style.border_size = Some(BorderSize::new(seed.number(5.0)));
        style.border_color = Some(seed.color(6));
        style.border_radius = Some(BorderRadius::new(seed.number(7.0)));
    }

    fn set_sizes_and_spacing(style: &mut ComputedStyle, seed: Seed) {
        style.width = Some(seed.pixels(8.0));
        style.height = Some(seed.pixels(9.0));
        style.min_width = Some(seed.pixels(10.0));
        style.min_height = Some(seed.pixels(11.0));
        style.max_width = Some(seed.pixels(12.0));
        style.max_height = Some(seed.pixels(13.0));
        style.padding = Some(seed.margin(14.0));
        style.margin = Some(seed.margin(15.0));
        style.gap = Some(seed.gap(16.0));
        style.column_gap = Some(seed.gap(17.0));
        style.row_gap = Some(seed.gap(18.0));
    }

    fn set_container_layout(style: &mut ComputedStyle, seed: Seed) {
        let flip = seed.flip();
        let aligns = seed.aligns();
        style.display = Some([DisplayMode::Flex, DisplayMode::Grid][flip]);
        style.flex_direction = Some([FlexDirection::Row, FlexDirection::Column][flip]);
        style.justify_content =
            Some([JustifyContent::SpaceBetween, JustifyContent::SpaceAround][flip]);
        style.align_items = Some(aligns[0]);
        style.justify_items = Some(aligns[1]);
        style.justify_self = Some(aligns[2]);
        style.align_content = Some([JustifyContent::SpaceEvenly, JustifyContent::Center][flip]);
        style.align_self = Some(aligns[3]);
    }

    fn set_grid(style: &mut ComputedStyle, seed: Seed) {
        style.grid_template_columns = Some(seed.tracks(19.0));
        style.grid_template_rows = Some(seed.tracks(20.0));
        style.grid_auto_columns = Some(seed.tracks(21.0));
        style.grid_auto_rows = Some(seed.tracks(22.0));
        style.grid_auto_flow = Some([GridAutoFlow::Column, GridAutoFlow::RowDense][seed.flip()]);
        style.grid_column = Some(seed.line(23));
        style.grid_row = Some(seed.line(24));
    }

    fn set_item(style: &mut ComputedStyle, seed: Seed) {
        style.position = Some([PositionType::Absolute, PositionType::Relative][seed.flip()]);
        style.opacity = Some(Opacity::new(seed.number(25.0) / 100.0).unwrap());
        style.flex_grow = Some(FlexGrow::new(seed.number(26.0)).unwrap());
        style.flex_shrink = Some(FlexShrink::new(seed.number(27.0)).unwrap());
        style.flex_basis = Some(seed.pixels(28.0));
    }

    #[test]
    fn test_merge_carries_every_declared_property_onto_an_empty_style() {
        let later = every_property(0);
        let mut style = ComputedStyle::default();

        style.merge_with(&later);

        assert_eq!(style, later);
    }

    #[test]
    fn test_merge_keeps_every_property_the_later_style_leaves_unset() {
        let earlier = every_property(0);
        let mut style = earlier.clone();

        style.merge_with(&ComputedStyle::default());

        assert_eq!(style, earlier);
    }

    #[test]
    fn test_merge_lets_the_later_style_win_on_every_property() {
        let later = every_property(40);
        let mut style = every_property(0);
        assert_ne!(style, later);

        style.merge_with(&later);

        assert_eq!(style, later);
    }
}
