use super::struct_def::ComputedStyle;
use crate::features::layout_engine::domain::{
    AlignItems, BoxMargin, FlexDirection, Gap, JustifyContent, PositionType,
};
use crate::features::styling::domain::grid_types::{
    DisplayMode, GridAutoFlow, GridLinePlacement, GridTrack,
};
use crate::features::styling::domain::values::{CssLength, FlexGrow, FlexShrink, Opacity};
use crate::shared::config::domain::{BorderRadius, BorderSize, FontFamily, FontSize};
use crate::shared::primitives::color::DrawingColor;

impl ComputedStyle {
    pub fn set_background(&mut self, bg: DrawingColor) {
        self.background = Some(bg);
    }
    pub fn set_color(&mut self, color: DrawingColor) {
        self.color = Some(color);
    }
    pub fn set_accent_color(&mut self, color: DrawingColor) {
        self.accent_color = Some(color);
    }
    pub fn set_font_family(&mut self, font_family: FontFamily) {
        self.font_family = Some(font_family);
    }
    pub const fn set_font_size(&mut self, font_size: FontSize) {
        self.font_size = Some(font_size);
    }
    pub const fn set_border_size(&mut self, border_size: BorderSize) {
        self.border_size = Some(border_size);
    }
    pub fn set_border_color(&mut self, border_color: DrawingColor) {
        self.border_color = Some(border_color);
    }
    pub const fn set_border_radius(&mut self, border_radius: BorderRadius) {
        self.border_radius = Some(border_radius);
    }
    pub const fn set_width(&mut self, width: CssLength) {
        self.width = Some(width);
    }
    pub const fn set_height(&mut self, height: CssLength) {
        self.height = Some(height);
    }
    pub const fn set_min_width(&mut self, min_width: CssLength) {
        self.min_width = Some(min_width);
    }
    pub const fn set_min_height(&mut self, min_height: CssLength) {
        self.min_height = Some(min_height);
    }
    pub const fn set_max_width(&mut self, max_width: CssLength) {
        self.max_width = Some(max_width);
    }
    pub const fn set_max_height(&mut self, max_height: CssLength) {
        self.max_height = Some(max_height);
    }
    pub const fn set_padding(&mut self, padding: BoxMargin) {
        self.padding = Some(padding);
    }
    pub const fn set_margin(&mut self, margin: BoxMargin) {
        self.margin = Some(margin);
    }
    pub const fn set_gap(&mut self, gap: Gap) {
        self.gap = Some(gap);
    }
    pub const fn set_column_gap(&mut self, column_gap: Gap) {
        self.column_gap = Some(column_gap);
    }
    pub const fn set_row_gap(&mut self, row_gap: Gap) {
        self.row_gap = Some(row_gap);
    }
    pub const fn set_display(&mut self, display: DisplayMode) {
        self.display = Some(display);
    }
    pub const fn set_flex_direction(&mut self, flex_direction: FlexDirection) {
        self.flex_direction = Some(flex_direction);
    }
    pub const fn set_justify_content(&mut self, justify_content: JustifyContent) {
        self.justify_content = Some(justify_content);
    }
    pub const fn set_align_items(&mut self, align_items: AlignItems) {
        self.align_items = Some(align_items);
    }
    pub const fn set_justify_items(&mut self, justify_items: AlignItems) {
        self.justify_items = Some(justify_items);
    }
    pub const fn set_justify_self(&mut self, justify_self: AlignItems) {
        self.justify_self = Some(justify_self);
    }
    pub const fn set_align_content(&mut self, align_content: JustifyContent) {
        self.align_content = Some(align_content);
    }
    pub fn set_grid_template_columns(&mut self, grid_template_columns: Vec<GridTrack>) {
        self.grid_template_columns = Some(grid_template_columns);
    }
    pub fn set_grid_template_rows(&mut self, grid_template_rows: Vec<GridTrack>) {
        self.grid_template_rows = Some(grid_template_rows);
    }
    pub fn set_grid_auto_columns(&mut self, grid_auto_columns: Vec<GridTrack>) {
        self.grid_auto_columns = Some(grid_auto_columns);
    }
    pub fn set_grid_auto_rows(&mut self, grid_auto_rows: Vec<GridTrack>) {
        self.grid_auto_rows = Some(grid_auto_rows);
    }
    pub const fn set_grid_auto_flow(&mut self, grid_auto_flow: GridAutoFlow) {
        self.grid_auto_flow = Some(grid_auto_flow);
    }
    pub const fn set_grid_column(&mut self, grid_column: GridLinePlacement) {
        self.grid_column = Some(grid_column);
    }
    pub const fn set_grid_row(&mut self, grid_row: GridLinePlacement) {
        self.grid_row = Some(grid_row);
    }
    pub const fn set_position(&mut self, position: PositionType) {
        self.position = Some(position);
    }
    pub const fn set_opacity(&mut self, opacity: Opacity) {
        self.opacity = Some(opacity);
    }
    pub const fn set_flex_grow(&mut self, flex_grow: FlexGrow) {
        self.flex_grow = Some(flex_grow);
    }
    pub const fn set_flex_shrink(&mut self, flex_shrink: FlexShrink) {
        self.flex_shrink = Some(flex_shrink);
    }
    pub const fn set_flex_basis(&mut self, flex_basis: CssLength) {
        self.flex_basis = Some(flex_basis);
    }
    pub const fn set_align_self(&mut self, align_self: AlignItems) {
        self.align_self = Some(align_self);
    }
}
