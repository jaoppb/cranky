//! Applies `display`, the gaps, and grid container properties (templates, auto tracks, auto
//! flow, item and content alignment).
use super::super::box_model::parse_length_str;
use super::super::flex::justify_content_from_css;
use super::declaration::split_declaration;
use super::tracks::parse_track_list;
use crate::features::layout_engine::domain::{AlignItems, Gap};
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
    let (name, val) = split_declaration(&full);
    match name {
        "display" => apply_display(style, val),
        "column-gap" => {
            style.set_column_gap(Gap::new(f64::from(parse_length_str(val))));
            true
        }
        "row-gap" => {
            style.set_row_gap(Gap::new(f64::from(parse_length_str(val))));
            true
        }
        _ => false,
    }
}

/// Sets `display` and reports it handled only for `grid`, `flex` and `none`.
fn apply_display(style: &mut ComputedStyle, val: &str) -> bool {
    let mode = match val {
        "grid" => DisplayMode::Grid,
        "flex" => DisplayMode::Flex,
        "none" => DisplayMode::None,
        _ => return false,
    };
    style.set_display(mode);
    true
}

pub(crate) fn apply_grid_container_properties(
    style: &mut ComputedStyle,
    prop: &Property<'_>,
) -> bool {
    let Ok(full) = prop.to_css_string(false, PrinterOptions::default()) else {
        return false;
    };
    let (name, val) = split_declaration(&full);
    match name {
        "grid-template-columns" => style.set_grid_template_columns(parse_track_list(val)),
        "grid-template-rows" => style.set_grid_template_rows(parse_track_list(val)),
        "grid-auto-columns" => style.set_grid_auto_columns(parse_track_list(val)),
        "grid-auto-rows" => style.set_grid_auto_rows(parse_track_list(val)),
        "grid-auto-flow" => style.set_grid_auto_flow(grid_auto_flow_from(val)),
        "justify-items" => style.set_justify_items(self_alignment_from(val)),
        "align-content" => style.set_align_content(justify_content_from_css(val)),
        _ => return false,
    }
    true
}

/// The auto-placement flow a serialized `grid-auto-flow` names, `Row` for anything else.
fn grid_auto_flow_from(val: &str) -> GridAutoFlow {
    match val {
        "column" => GridAutoFlow::Column,
        "row dense" | "dense row" => GridAutoFlow::RowDense,
        "column dense" | "dense column" => GridAutoFlow::ColumnDense,
        _ => GridAutoFlow::Row,
    }
}

/// The alignment a whole `justify-items`/`justify-self` keyword names, `Start` for any other.
pub(super) fn self_alignment_from(val: &str) -> AlignItems {
    match val {
        "center" => AlignItems::Center,
        "end" | "flex-end" => AlignItems::End,
        "stretch" => AlignItems::Stretch,
        _ => AlignItems::Start,
    }
}
