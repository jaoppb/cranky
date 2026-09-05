pub mod box_model;
pub mod color_font;
pub mod flex;
pub mod grid;

use box_model::{
    apply_border_properties, apply_margin_properties, apply_padding_properties,
    apply_sizing_properties,
};
use color_font::apply_color_and_font;
use flex::{apply_flex_container_properties, apply_flex_item_properties};
use grid::{
    apply_display_and_gap_properties, apply_grid_container_properties,
    apply_grid_item_properties,
};

use crate::features::styling::domain::ComputedStyle;
use lightningcss::declaration::DeclarationBlock;

#[must_use]
pub fn parse_declarations(declarations: &DeclarationBlock) -> ComputedStyle {
    let mut style = ComputedStyle::default();

    for prop in declarations
        .declarations
        .iter()
        .chain(declarations.important_declarations.iter())
    {
        let _ = apply_color_and_font(&mut style, prop)
            || apply_border_properties(&mut style, prop)
            || apply_padding_properties(&mut style, prop)
            || apply_margin_properties(&mut style, prop)
            || apply_sizing_properties(&mut style, prop)
            || apply_display_and_gap_properties(&mut style, prop)
            || apply_flex_container_properties(&mut style, prop)
            || apply_flex_item_properties(&mut style, prop)
            || apply_grid_container_properties(&mut style, prop)
            || apply_grid_item_properties(&mut style, prop);
    }

    style
}
