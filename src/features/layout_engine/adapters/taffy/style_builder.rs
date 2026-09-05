use super::grid_converter::{
    grid_auto_flow_to_taffy, grid_line_placement_to_taffy, grid_track_to_template_component,
    grid_track_to_track_sizing_function,
};
use crate::features::layout_engine::domain::{StyledNode, TextMeasurer};
use crate::features::styling::domain::{DisplayMode, Orientation};
use crate::shared::primitives::geometry::Size;
use taffy::geometry::Size as TaffySize;
use taffy::style::{Dimension, LengthPercentage, Style};

#[allow(
    clippy::as_conversions,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::too_many_lines
)]
pub fn node_to_style(node: &StyledNode, measurer: &mut dyn TextMeasurer) -> Style {
    let computed = node.style();
    let col_gap = computed
        .column_gap()
        .or_else(|| computed.gap())
        .map_or_else(
            || LengthPercentage::length(0.0),
            |g| LengthPercentage::length(g.value() as f32),
        );
    let row_gap = computed.row_gap().or_else(|| computed.gap()).map_or_else(
        || LengthPercentage::length(0.0),
        |g| LengthPercentage::length(g.value() as f32),
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
            if let Some(ji) = computed.justify_items() {
                style.justify_items = Some(ji.into());
            }
            if let Some(ac) = computed.align_content() {
                style.align_content = Some(ac.into());
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

    if let Some(col) = computed.grid_column() {
        style.grid_column = grid_line_placement_to_taffy(*col);
    }
    if let Some(row) = computed.grid_row() {
        style.grid_row = grid_line_placement_to_taffy(*row);
    }
    if let Some(js) = computed.justify_self() {
        style.justify_self = Some(js.into());
    }

    if let Some(w) = computed.width() {
        style.size.width = w.into();
    }
    if let Some(h) = computed.height() {
        style.size.height = h.into();
    }
    if let Some(mw) = computed.min_width() {
        style.min_size.width = mw.into();
    }
    if let Some(mw) = computed.max_width() {
        style.max_size.width = mw.into();
    }
    if let Some(mh) = computed.min_height() {
        style.min_size.height = mh.into();
    }
    if let Some(mh) = computed.max_height() {
        style.max_size.height = mh.into();
    }
    if let Some(fg) = computed.flex_grow() {
        style.flex_grow = fg.value();
    }
    if let Some(fs) = computed.flex_shrink() {
        style.flex_shrink = fs.value();
    }
    if let Some(fb) = computed.flex_basis() {
        style.flex_basis = fb.into();
    }
    if let Some(as_) = computed.align_self() {
        style.align_self = Some(as_.into());
    }

    match node {
        StyledNode::Flex { .. } | StyledNode::Grid { .. } => style,
        StyledNode::Text { text, style: s, .. } => {
            let text_size = measurer.measure(text.as_str(), s.font_family(), s.font_size());
            if computed.width().is_none() {
                style.size.width = Dimension::length(text_size.width() as f32);
            }
            if computed.height().is_none() {
                style.size.height = Dimension::length(text_size.height() as f32);
            }
            style
        }
        StyledNode::Progress { orientation, .. } => {
            let default_size = match orientation {
                Orientation::Horizontal => Size::new(40, 8),
                Orientation::Vertical => Size::new(8, 40),
            };
            if computed.width().is_none() {
                style.size.width = Dimension::length(default_size.width() as f32);
            }
            if computed.height().is_none() {
                style.size.height = Dimension::length(default_size.height() as f32);
            }
            style
        }
        StyledNode::Rect { .. } => {
            if computed.width().is_none() {
                style.size.width = Dimension::length(10.0);
            }
            if computed.height().is_none() {
                style.size.height = Dimension::length(10.0);
            }
            style
        }
        StyledNode::Image { .. } => {
            if computed.width().is_none() {
                style.size.width = Dimension::length(24.0);
            }
            if computed.height().is_none() {
                style.size.height = Dimension::length(24.0);
            }
            style
        }
        StyledNode::Module { key, .. } => {
            if let Some(size) = measurer.measure_module(key) {
                if computed.width().is_none() {
                    style.size.width = Dimension::length(size.width() as f32);
                }
                if computed.height().is_none() {
                    style.size.height = Dimension::length(size.height() as f32);
                }
            }
            style
        }
    }
}
