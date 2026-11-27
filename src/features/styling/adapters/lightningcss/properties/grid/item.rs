//! Applies grid item properties: `grid-row`/`grid-column` placement and `justify-self`.
use super::container::self_alignment_from;
use super::declaration::split_declaration;
use super::placement::{parse_line_placement, parse_single_placement};
use crate::features::styling::domain::{ComputedStyle, GridLinePlacement, GridPlacement};
use lightningcss::properties::Property;
use lightningcss::stylesheet::PrinterOptions;

pub(crate) fn apply_grid_item_properties(style: &mut ComputedStyle, prop: &Property<'_>) -> bool {
    let Ok(full) = prop.to_css_string(false, PrinterOptions::default()) else {
        return false;
    };
    let (name, val) = split_declaration(&full);
    match name {
        "grid-column" => style.set_grid_column(parse_line_placement(val)),
        "grid-column-start" => style.set_grid_column(with_start(style.grid_column(), val)),
        "grid-column-end" => style.set_grid_column(with_end(style.grid_column(), val)),
        "grid-row" => style.set_grid_row(parse_line_placement(val)),
        "grid-row-start" => style.set_grid_row(with_start(style.grid_row(), val)),
        "grid-row-end" => style.set_grid_row(with_end(style.grid_row(), val)),
        "justify-self" => style.set_justify_self(self_alignment_from(val)),
        _ => return false,
    }
    true
}

/// `current` (auto at both ends when undeclared) with its start replaced by `val`.
fn with_start(current: Option<&GridLinePlacement>, val: &str) -> GridLinePlacement {
    let start = parse_single_placement(val);
    let end = current
        .copied()
        .map_or(GridPlacement::Auto, GridLinePlacement::end);
    GridLinePlacement::new(start, end)
}

/// `current` (auto at both ends when undeclared) with its end replaced by `val`.
fn with_end(current: Option<&GridLinePlacement>, val: &str) -> GridLinePlacement {
    let end = parse_single_placement(val);
    let start = current
        .copied()
        .map_or(GridPlacement::Auto, GridLinePlacement::start);
    GridLinePlacement::new(start, end)
}
