use crate::features::layout_engine::domain::{StyledNode, TextMeasurer};
use crate::features::styling::domain::{ComputedStyle, Orientation};
use crate::shared::primitives::geometry::Size;
use crate::shared::rendering::ports::text::TextFont;
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
            |padding| padded_dimension(intrinsic.width(), padding.left(), padding.right()),
        );
    }
    if computed.height().is_none() {
        style.size.height = pad.map_or_else(
            || size_to_dimension(intrinsic.height()),
            |padding| padded_dimension(intrinsic.height(), padding.top(), padding.bottom()),
        );
    }
}

pub(super) fn apply_node_dimensions(
    style: &mut Style,
    node: &StyledNode,
    measurer: &mut dyn TextMeasurer,
) {
    let computed = node.style();
    // A module's reported size is the child's own outer box, so the
    // embed node's padding is not added on top of it.
    if let StyledNode::Module { key, .. } = node {
        if let Some(size) = measurer.measure_module(key) {
            apply_module_size(style, computed, size);
        }
        return;
    }
    if let Some(intrinsic) = leaf_intrinsic_size(node, measurer) {
        apply_intrinsic(style, computed, intrinsic);
    }
}

/// The content size of a leaf before padding: its measured text, or the default size of a
/// progress bar, rect or image. Containers have none; a module never gets here, as
/// `apply_node_dimensions` sizes it from its report, and is listed only to keep the match
/// exhaustive.
fn leaf_intrinsic_size(node: &StyledNode, measurer: &mut dyn TextMeasurer) -> Option<Size> {
    match node {
        StyledNode::Flex { .. } | StyledNode::Grid { .. } | StyledNode::Module { .. } => None,
        StyledNode::Text {
            text,
            style: text_style,
            ..
        } => Some(measurer.measure(
            text.as_str(),
            TextFont::new(text_style.font_family(), text_style.font_size()),
        )),
        StyledNode::Progress { orientation, .. } => Some(match orientation {
            Orientation::Horizontal => Size::new(40, 8),
            Orientation::Vertical => Size::new(8, 40),
        }),
        StyledNode::Rect { .. } => Some(Size::new(10, 10)),
        StyledNode::Image { .. } => Some(Size::new(24, 24)),
    }
}

/// Sizes an embedded module's node to the `size` the module reported, on each axis its style
/// leaves unset.
fn apply_module_size(style: &mut Style, computed: &ComputedStyle, size: Size) {
    if computed.width().is_none() {
        style.size.width = size_to_dimension(size.width());
    }
    if computed.height().is_none() {
        style.size.height = size_to_dimension(size.height());
    }
}
