use super::grid_converter::{
    grid_auto_flow_to_taffy, grid_line_placement_to_taffy, grid_track_to_template_component,
    grid_track_to_track_sizing_function,
};
use super::leaf_size::apply_node_dimensions;
use crate::features::layout_engine::domain::{StyledNode, TextMeasurer};
use crate::features::styling::domain::{ComputedStyle, DisplayMode};
use crate::utils::f64_to_f32;
use taffy::geometry::Size as TaffySize;
use taffy::style::{LengthPercentage, Style};

fn apply_display_mode(style: &mut Style, computed: &ComputedStyle, node: &StyledNode) {
    let display_mode = match (computed.display(), node) {
        (Some(mode), _) => mode,
        (None, StyledNode::Grid { .. }) => DisplayMode::Grid,
        (None, _) => DisplayMode::Flex,
    };

    match display_mode {
        DisplayMode::Grid => {
            style.display = taffy::style::Display::Grid;
            if let Some(cols) = computed.grid_template_columns() {
                style.grid_template_columns =
                    cols.iter().map(grid_track_to_template_component).collect();
            }
            if let Some(rows) = computed.grid_template_rows() {
                style.grid_template_rows =
                    rows.iter().map(grid_track_to_template_component).collect();
            }
            if let Some(auto_cols) = computed.grid_auto_columns() {
                style.grid_auto_columns = auto_cols
                    .iter()
                    .map(grid_track_to_track_sizing_function)
                    .collect();
            }
            if let Some(auto_rows) = computed.grid_auto_rows() {
                style.grid_auto_rows = auto_rows
                    .iter()
                    .map(grid_track_to_track_sizing_function)
                    .collect();
            }
            if let Some(flow) = computed.grid_auto_flow() {
                style.grid_auto_flow = grid_auto_flow_to_taffy(flow);
            }
            if let Some(justify_items) = computed.justify_items() {
                style.justify_items = Some(justify_items.into());
            }
            if let Some(align_content) = computed.align_content() {
                style.align_content = Some(align_content.into());
            }
        }
        DisplayMode::Flex => {
            style.display = taffy::style::Display::Flex;
            style.flex_direction = computed.flex_direction().unwrap_or_default().into();
        }
        DisplayMode::None => {
            style.display = taffy::style::Display::None;
        }
    }
}

fn apply_sizing_and_flex(style: &mut Style, computed: &ComputedStyle) {
    if let Some(col) = computed.grid_column() {
        style.grid_column = grid_line_placement_to_taffy(*col);
    }
    if let Some(row) = computed.grid_row() {
        style.grid_row = grid_line_placement_to_taffy(*row);
    }
    if let Some(justify_self) = computed.justify_self() {
        style.justify_self = Some(justify_self.into());
    }

    if let Some(width) = computed.width() {
        style.size.width = width.into();
    }
    if let Some(height) = computed.height() {
        style.size.height = height.into();
    }
    if let Some(min_width) = computed.min_width() {
        style.min_size.width = min_width.into();
    }
    if let Some(max_width) = computed.max_width() {
        style.max_size.width = max_width.into();
    }
    if let Some(min_height) = computed.min_height() {
        style.min_size.height = min_height.into();
    }
    if let Some(max_height) = computed.max_height() {
        style.max_size.height = max_height.into();
    }
    if let Some(flex_grow) = computed.flex_grow() {
        style.flex_grow = flex_grow.value();
    }
    if let Some(flex_shrink) = computed.flex_shrink() {
        style.flex_shrink = flex_shrink.value();
    }
    if let Some(flex_basis) = computed.flex_basis() {
        style.flex_basis = flex_basis.into();
    }
    if let Some(as_) = computed.align_self() {
        style.align_self = Some(as_.into());
    }
}

#[must_use]
pub(super) fn node_to_style(node: &StyledNode, measurer: &mut dyn TextMeasurer) -> Style {
    let computed = node.style();
    let col_gap = computed
        .column_gap()
        .or_else(|| computed.gap())
        .map_or_else(
            || LengthPercentage::length(0.0),
            |gap| LengthPercentage::length(f64_to_f32(gap.value())),
        );
    let row_gap = computed.row_gap().or_else(|| computed.gap()).map_or_else(
        || LengthPercentage::length(0.0),
        |gap| LengthPercentage::length(f64_to_f32(gap.value())),
    );

    let mut style = Style {
        position: computed.position().unwrap_or_default().into(),
        padding: computed
            .padding()
            .map_or_else(taffy::geometry::Rect::zero, Into::into),
        margin: computed
            .margin()
            .map_or_else(taffy::geometry::Rect::zero, Into::into),
        gap: TaffySize {
            width: col_gap,
            height: row_gap,
        },
        justify_content: computed.justify_content().map(Into::into),
        align_items: computed.align_items().map(Into::into),
        ..Default::default()
    };

    apply_display_mode(&mut style, computed, node);
    apply_sizing_and_flex(&mut style, computed);
    apply_node_dimensions(&mut style, node, computed, measurer);

    style
}
