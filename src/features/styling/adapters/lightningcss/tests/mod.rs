mod basic;
mod cascade;
mod embed_sizing;
mod grid_selectors;
mod inheritance;
mod layout_integration;
mod property_appliers;
mod property_box_model;
mod property_color_font;
mod property_flex;
mod property_grid_container;
mod property_grid_item;
mod rule_entries;
mod selector_compilation;
mod selectors;
mod support;
mod text_align;
mod vdom_bridge;

use crate::features::styling::adapters::lightningcss::LightningCssAdapter;
use crate::features::styling::domain::{
    ComputedStyle, ElementIdentity, ElementQuery, InheritedStyle, StyleSheetName,
};
use crate::features::styling::ports::CssParserPort;

/// The style a `flex` element ends up with when `declarations` is the body
/// of the only rule that matches it.
fn declared(declarations: &str) -> ComputedStyle {
    let css = format!("flex {{ {declarations} }}");
    let parsed = LightningCssAdapter::new()
        .parse_stylesheet(StyleSheetName::new("test").unwrap(), &css)
        .unwrap();
    let query = ElementQuery::new(ElementIdentity::new("flex", None, &[]), &[], None);
    parsed.resolve_style(&query, &InheritedStyle::default()).0
}
