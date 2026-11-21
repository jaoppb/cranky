//! Read accessors on [`ComputedStyle`], used by layout and paint.
use super::struct_def::ComputedStyle;
use crate::features::layout_engine::domain::{
    AlignItems, BoxMargin, FlexDirection, Gap, JustifyContent, PositionType,
};
use crate::features::styling::domain::grid_types::{
    DisplayMode, GridAutoFlow, GridLinePlacement, GridTrack,
};
#[cfg(test)]
use crate::features::styling::domain::values::Opacity;
use crate::features::styling::domain::values::{CssLength, FlexGrow, FlexShrink};
use crate::shared::config::domain::{BorderRadius, BorderSize, FontFamily, FontSize};
use crate::shared::primitives::TextAlign;
use crate::shared::primitives::color::DrawingColor;

impl ComputedStyle {
    #[must_use]
    pub(crate) const fn background(&self) -> Option<&DrawingColor> {
        self.background.as_ref()
    }
    #[must_use]
    pub(crate) const fn color(&self) -> Option<&DrawingColor> {
        self.color.as_ref()
    }
    #[must_use]
    pub(crate) const fn accent_color(&self) -> Option<&DrawingColor> {
        self.accent_color.as_ref()
    }
    #[must_use]
    pub(crate) const fn font_family(&self) -> Option<&FontFamily> {
        self.font_family.as_ref()
    }
    #[must_use]
    pub(crate) const fn text_align(&self) -> Option<TextAlign> {
        self.text_align
    }
    #[must_use]
    pub(crate) const fn font_size(&self) -> Option<FontSize> {
        self.font_size
    }
    #[must_use]
    pub(crate) const fn border_size(&self) -> Option<BorderSize> {
        self.border_size
    }
    #[must_use]
    pub(crate) const fn border_color(&self) -> Option<&DrawingColor> {
        self.border_color.as_ref()
    }
    #[must_use]
    pub(crate) const fn border_radius(&self) -> Option<BorderRadius> {
        self.border_radius
    }
    #[must_use]
    pub(crate) const fn width(&self) -> Option<CssLength> {
        self.width
    }
    #[must_use]
    pub(crate) const fn height(&self) -> Option<CssLength> {
        self.height
    }
    #[must_use]
    pub(crate) const fn min_width(&self) -> Option<CssLength> {
        self.min_width
    }
    #[must_use]
    pub(crate) const fn min_height(&self) -> Option<CssLength> {
        self.min_height
    }
    #[must_use]
    pub(crate) const fn max_width(&self) -> Option<CssLength> {
        self.max_width
    }
    #[must_use]
    pub(crate) const fn max_height(&self) -> Option<CssLength> {
        self.max_height
    }
    #[must_use]
    pub(crate) const fn padding(&self) -> Option<&BoxMargin> {
        self.padding.as_ref()
    }
    #[must_use]
    pub(crate) const fn margin(&self) -> Option<&BoxMargin> {
        self.margin.as_ref()
    }
    #[must_use]
    pub(crate) const fn gap(&self) -> Option<&Gap> {
        self.gap.as_ref()
    }
    #[must_use]
    pub(crate) const fn column_gap(&self) -> Option<&Gap> {
        self.column_gap.as_ref()
    }
    #[must_use]
    pub(crate) const fn row_gap(&self) -> Option<&Gap> {
        self.row_gap.as_ref()
    }
    #[must_use]
    pub(crate) const fn display(&self) -> Option<DisplayMode> {
        self.display
    }
    #[must_use]
    pub(crate) const fn flex_direction(&self) -> Option<FlexDirection> {
        self.flex_direction
    }
    #[must_use]
    pub(crate) const fn justify_content(&self) -> Option<JustifyContent> {
        self.justify_content
    }
    #[must_use]
    pub(crate) const fn align_items(&self) -> Option<AlignItems> {
        self.align_items
    }
    #[must_use]
    pub(crate) const fn justify_items(&self) -> Option<AlignItems> {
        self.justify_items
    }
    #[must_use]
    pub(crate) const fn justify_self(&self) -> Option<AlignItems> {
        self.justify_self
    }
    #[must_use]
    pub(crate) const fn align_content(&self) -> Option<JustifyContent> {
        self.align_content
    }
    #[must_use]
    pub(crate) fn grid_template_columns(&self) -> Option<&[GridTrack]> {
        self.grid_template_columns.as_deref()
    }
    #[must_use]
    pub(crate) fn grid_template_rows(&self) -> Option<&[GridTrack]> {
        self.grid_template_rows.as_deref()
    }
    #[must_use]
    pub(crate) fn grid_auto_columns(&self) -> Option<&[GridTrack]> {
        self.grid_auto_columns.as_deref()
    }
    #[must_use]
    pub(crate) fn grid_auto_rows(&self) -> Option<&[GridTrack]> {
        self.grid_auto_rows.as_deref()
    }
    #[must_use]
    pub(crate) const fn grid_auto_flow(&self) -> Option<GridAutoFlow> {
        self.grid_auto_flow
    }
    #[must_use]
    pub(crate) const fn grid_column(&self) -> Option<&GridLinePlacement> {
        self.grid_column.as_ref()
    }
    #[must_use]
    pub(crate) const fn grid_row(&self) -> Option<&GridLinePlacement> {
        self.grid_row.as_ref()
    }
    #[must_use]
    pub(crate) const fn position(&self) -> Option<PositionType> {
        self.position
    }
    #[cfg(test)]
    #[must_use]
    pub(crate) const fn opacity(&self) -> Option<Opacity> {
        self.opacity
    }
    #[must_use]
    pub(crate) const fn flex_grow(&self) -> Option<FlexGrow> {
        self.flex_grow
    }
    #[must_use]
    pub(crate) const fn flex_shrink(&self) -> Option<FlexShrink> {
        self.flex_shrink
    }
    #[must_use]
    pub(crate) const fn flex_basis(&self) -> Option<CssLength> {
        self.flex_basis
    }
    #[must_use]
    pub(crate) const fn align_self(&self) -> Option<AlignItems> {
        self.align_self
    }
}
