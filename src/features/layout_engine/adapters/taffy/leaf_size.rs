use crate::features::layout_engine::domain::{StyledNode, TextMeasurer};
use crate::features::styling::domain::{ComputedStyle, Orientation};
use crate::shared::primitives::geometry::Size;
use crate::utils::f64_to_f32;
use taffy::style::{Dimension, Style};

pub(super) fn size_to_dimension(val: u32) -> Dimension {
    Dimension::length(f32::from(u16::try_from(val).unwrap_or(u16::MAX)))
}

/// A leaf's intrinsic size is its content *plus* its padding. Taffy sizes
/// boxes border-box, so handing it the bare content size would let padding
/// eat into the glyph instead of growing the box.
fn padded_dimension(content: u32, before: f64, after: f64) -> Dimension {
    let padding = f64_to_f32(before.max(0.0) + after.max(0.0));
    let content = f32::from(u16::try_from(content).unwrap_or(u16::MAX));
    Dimension::length(content + padding)
}

fn apply_intrinsic(style: &mut Style, computed: &ComputedStyle, intrinsic: Size) {
    let pad = computed.padding();
    if computed.width().is_none() {
        style.size.width = pad.map_or_else(
            || size_to_dimension(intrinsic.width()),
            |p| padded_dimension(intrinsic.width(), p.left(), p.right()),
        );
    }
    if computed.height().is_none() {
        style.size.height = pad.map_or_else(
            || size_to_dimension(intrinsic.height()),
            |p| padded_dimension(intrinsic.height(), p.top(), p.bottom()),
        );
    }
}

pub(super) fn apply_node_dimensions(
    style: &mut Style,
    node: &StyledNode,
    computed: &ComputedStyle,
    measurer: &mut dyn TextMeasurer,
) {
    match node {
        StyledNode::Flex { .. } | StyledNode::Grid { .. } => {}
        StyledNode::Text { text, style: s, .. } => {
            let text_size = measurer.measure(text.as_str(), s.font_family(), s.font_size());
            apply_intrinsic(style, computed, text_size);
        }
        StyledNode::Progress { orientation, .. } => {
            let default_size = match orientation {
                Orientation::Horizontal => Size::new(40, 8),
                Orientation::Vertical => Size::new(8, 40),
            };
            apply_intrinsic(style, computed, default_size);
        }
        StyledNode::Rect { .. } => apply_intrinsic(style, computed, Size::new(10, 10)),
        StyledNode::Image { .. } => apply_intrinsic(style, computed, Size::new(24, 24)),
        // A module's reported size is the child's own outer box, so the
        // embed node's padding is not added on top of it.
        StyledNode::Module { key, .. } => {
            if let Some(size) = measurer.measure_module(key) {
                if computed.width().is_none() {
                    style.size.width = size_to_dimension(size.width());
                }
                if computed.height().is_none() {
                    style.size.height = size_to_dimension(size.height());
                }
            }
        }
    }
}
