//! Calls each property applier directly, so a test sees both what it writes
//! and whether it reports the declaration as handled.
mod border_sizing;
mod color_font;
mod flex;
mod grid;
mod spacing;
mod tracks;

use crate::features::styling::domain::ComputedStyle;
use lightningcss::properties::Property;
use lightningcss::rules::CssRule;
use lightningcss::stylesheet::{ParserOptions, StyleSheet};

/// A property applier, as `parse_property_list` chains them.
type Applier = fn(&mut ComputedStyle, &Property<'_>) -> bool;

/// The style `applier` leaves after it sees each declaration of
/// `flex { <declarations> }` in turn, and whether it reported each one as
/// handled.
fn applied(applier: Applier, declarations: &str) -> (ComputedStyle, Vec<bool>) {
    let css = format!("flex {{ {declarations} }}");
    let sheet = StyleSheet::parse(&css, ParserOptions::default()).unwrap();
    let CssRule::Style(style_rule) = sheet.rules.0.first().unwrap() else {
        panic!("expected a style rule");
    };
    let mut style = ComputedStyle::default();
    let handled = style_rule
        .declarations
        .declarations
        .iter()
        .map(|property| applier(&mut style, property))
        .collect();
    (style, handled)
}
