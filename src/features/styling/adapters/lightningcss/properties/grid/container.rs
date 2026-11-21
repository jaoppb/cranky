//! Applies `display`, the gaps, and grid container properties (templates, auto tracks, auto
//! flow, item and content alignment).
use super::super::box_model::parse_length_str;
use super::tracks::parse_track_list;
use crate::features::layout_engine::domain::{AlignItems, Gap, JustifyContent};
use crate::features::styling::domain::{ComputedStyle, DisplayMode, GridAutoFlow};
use lightningcss::properties::Property;
use lightningcss::stylesheet::PrinterOptions;

pub(crate) fn apply_display_and_gap_properties(
    style: &mut ComputedStyle,
    prop: &Property<'_>,
) -> bool {
    let Ok(full) = prop.to_css_string(false, PrinterOptions::default()) else {
        return false;
    };
    let (name, val) = full
        .split_once(':')
        .map_or(("", full.as_str()), |(property, value)| {
            (property.trim(), value.trim())
        });

    if name == "display" {
        let mode = match val {
            "grid" => Some(DisplayMode::Grid),
            "flex" => Some(DisplayMode::Flex),
            "none" => Some(DisplayMode::None),
            _ => None,
        };
        if let Some(display) = mode {
            style.set_display(display);
            return true;
        }
    } else if name == "column-gap" {
        let len = parse_length_str(val);
        style.set_column_gap(Gap::new(f64::from(len)));
        return true;
    } else if name == "row-gap" {
        let len = parse_length_str(val);
        style.set_row_gap(Gap::new(f64::from(len)));
        return true;
    }
    false
}

pub(crate) fn apply_grid_container_properties(
    style: &mut ComputedStyle,
    prop: &Property<'_>,
) -> bool {
    let Ok(full) = prop.to_css_string(false, PrinterOptions::default()) else {
        return false;
    };
    let (name, val) = full
        .split_once(':')
        .map_or(("", full.as_str()), |(property, value)| {
            (property.trim(), value.trim())
        });

    match name {
        "grid-template-columns" => {
            let tracks = parse_track_list(val);
            style.set_grid_template_columns(tracks);
            true
        }
        "grid-template-rows" => {
            let tracks = parse_track_list(val);
            style.set_grid_template_rows(tracks);
            true
        }
        "grid-auto-columns" => {
            let tracks = parse_track_list(val);
            style.set_grid_auto_columns(tracks);
            true
        }
        "grid-auto-rows" => {
            let tracks = parse_track_list(val);
            style.set_grid_auto_rows(tracks);
            true
        }
        "grid-auto-flow" => {
            let flow = match val {
                "column" => GridAutoFlow::Column,
                "row dense" | "dense row" => GridAutoFlow::RowDense,
                "column dense" | "dense column" => GridAutoFlow::ColumnDense,
                _ => GridAutoFlow::Row,
            };
            style.set_grid_auto_flow(flow);
            true
        }
        "justify-items" => {
            let align = match val {
                "center" => AlignItems::Center,
                "end" | "flex-end" => AlignItems::End,
                "stretch" => AlignItems::Stretch,
                _ => AlignItems::Start,
            };
            style.set_justify_items(align);
            true
        }
        "align-content" => {
            let content_alignment = if val.contains("space-between") {
                JustifyContent::SpaceBetween
            } else if val.contains("space-around") {
                JustifyContent::SpaceAround
            } else if val.contains("space-evenly") {
                JustifyContent::SpaceEvenly
            } else if val.contains("center") {
                JustifyContent::Center
            } else if val.contains("end") || val.contains("flex-end") {
                JustifyContent::End
            } else {
                JustifyContent::Start
            };
            style.set_align_content(content_alignment);
            true
        }
        _ => false,
    }
}
