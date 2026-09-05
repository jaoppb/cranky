use super::placement::{parse_line_placement, parse_single_placement};
use crate::features::layout_engine::domain::AlignItems;
use crate::features::styling::domain::{ComputedStyle, GridLinePlacement, GridPlacement};
use lightningcss::properties::Property;
use lightningcss::stylesheet::PrinterOptions;

pub fn apply_grid_item_properties(style: &mut ComputedStyle, prop: &Property) -> bool {
    let Ok(full) = prop.to_css_string(false, PrinterOptions::default()) else {
        return false;
    };
    let (name, val) = full
        .split_once(':')
        .map_or(("", full.as_str()), |(k, v)| (k.trim(), v.trim()));

    match name {
        "grid-column" => {
            style.set_grid_column(parse_line_placement(val));
            true
        }
        "grid-column-start" => {
            let start = parse_single_placement(val);
            let end = style
                .grid_column()
                .map_or(GridPlacement::Auto, GridLinePlacement::end);
            style.set_grid_column(GridLinePlacement::new(start, end));
            true
        }
        "grid-column-end" => {
            let end = parse_single_placement(val);
            let start = style
                .grid_column()
                .map_or(GridPlacement::Auto, GridLinePlacement::start);
            style.set_grid_column(GridLinePlacement::new(start, end));
            true
        }
        "grid-row" => {
            style.set_grid_row(parse_line_placement(val));
            true
        }
        "grid-row-start" => {
            let start = parse_single_placement(val);
            let end = style
                .grid_row()
                .map_or(GridPlacement::Auto, GridLinePlacement::end);
            style.set_grid_row(GridLinePlacement::new(start, end));
            true
        }
        "grid-row-end" => {
            let end = parse_single_placement(val);
            let start = style
                .grid_row()
                .map_or(GridPlacement::Auto, GridLinePlacement::start);
            style.set_grid_row(GridLinePlacement::new(start, end));
            true
        }
        "justify-self" => {
            let align = match val {
                "center" => AlignItems::Center,
                "end" | "flex-end" => AlignItems::End,
                "stretch" => AlignItems::Stretch,
                _ => AlignItems::Start,
            };
            style.set_justify_self(align);
            true
        }
        _ => false,
    }
}
