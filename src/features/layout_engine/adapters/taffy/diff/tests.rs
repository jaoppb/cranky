use super::diff;
use crate::features::layout_engine::adapters::taffy::reconciler::{LayoutState, Patch, Update};
use crate::features::layout_engine::domain::{NodePath, StyledNode, SurfaceSpace, TextMeasurer};
use crate::features::styling::domain::{ComputedStyle, CssLength, Orientation, ProgressValue};
use crate::features::vdom::domain::TextContent;
use crate::shared::primitives::BinaryData;
use crate::shared::primitives::geometry::Size;
use crate::shared::rendering::ports::text::TextFont;
use taffy::tree::NodeId;

struct FixedMeasurer;

impl TextMeasurer for FixedMeasurer {
    fn measure(&mut self, _text: &str, _font: TextFont<'_>) -> Size {
        Size::new(10, 10)
    }
}

fn state(layout: StyledNode) -> LayoutState {
    LayoutState {
        root_node: NodeId::from(7_u64),
        layout,
        children: Vec::new(),
    }
}

fn wide() -> ComputedStyle {
    let mut style = ComputedStyle::default();
    style.set_width(CssLength::Pixels(40.0));
    style
}

fn text(content: &str, style: ComputedStyle) -> StyledNode {
    StyledNode::Text {
        path: NodePath::root_in(SurfaceSpace::Bar),
        node_key: None,
        text: TextContent::new(content.to_string()),
        style,
        on_click: None,
        on_hover: None,
        tooltip: None,
        popup: None,
        panel: None,
    }
}

fn progress(value: f32, orientation: Orientation, style: ComputedStyle) -> StyledNode {
    StyledNode::Progress {
        path: NodePath::root_in(SurfaceSpace::Bar),
        node_key: None,
        value: ProgressValue::new(value).unwrap(),
        orientation,
        style,
        on_click: None,
        on_hover: None,
        tooltip: None,
        popup: None,
        panel: None,
    }
}

fn rect(style: ComputedStyle) -> StyledNode {
    StyledNode::Rect {
        path: NodePath::root_in(SurfaceSpace::Bar),
        node_key: None,
        style,
        on_click: None,
        on_hover: None,
        tooltip: None,
        popup: None,
        panel: None,
    }
}

fn image(pixel_size: Size, style: ComputedStyle) -> StyledNode {
    StyledNode::Image {
        path: NodePath::root_in(SurfaceSpace::Bar),
        node_key: None,
        data: BinaryData::new(vec![0_u8; 4]),
        pixel_size,
        style,
        tooltip: None,
        popup: None,
        panel: None,
    }
}

/// `Some(true)` when the leaf is kept, `Some(false)` when it is updated in
/// place with a fresh style and no child list, `None` for any other patch.
fn kept(old: &StyledNode, new: &StyledNode) -> Option<bool> {
    let old_state = state(old.clone());
    match diff(&old_state, new, &mut FixedMeasurer) {
        Patch::Keep(kept_state) => {
            assert!(std::ptr::eq(kept_state, &raw const old_state));
            Some(true)
        }
        Patch::Update(Update {
            old_state: updated,
            new_layout,
            style,
            children,
        }) => {
            assert!(std::ptr::eq(updated, &raw const old_state));
            assert_eq!(new_layout, new);
            assert!(style.is_some());
            assert!(children.is_none());
            Some(false)
        }
        Patch::Replace { .. } | Patch::Create { .. } => None,
    }
}

#[test]
fn identical_leaves_are_kept() {
    let leaves = [
        text("a", ComputedStyle::default()),
        progress(0.5, Orientation::Horizontal, ComputedStyle::default()),
        rect(ComputedStyle::default()),
        image(Size::new(2, 2), ComputedStyle::default()),
    ];
    for leaf in &leaves {
        assert_eq!(kept(leaf, leaf), Some(true), "{leaf:?}");
    }
}

#[test]
fn a_text_leaf_is_updated_when_its_text_or_style_changes() {
    let old = text("a", ComputedStyle::default());
    assert_eq!(
        kept(&old, &text("b", ComputedStyle::default())),
        Some(false)
    );
    assert_eq!(kept(&old, &text("a", wide())), Some(false));
}

#[test]
fn a_progress_leaf_is_updated_when_its_value_orientation_or_style_changes() {
    let old = progress(0.5, Orientation::Horizontal, ComputedStyle::default());
    let changed = [
        progress(0.75, Orientation::Horizontal, ComputedStyle::default()),
        progress(0.5, Orientation::Vertical, ComputedStyle::default()),
        progress(0.5, Orientation::Horizontal, wide()),
    ];
    for new in &changed {
        assert_eq!(kept(&old, new), Some(false), "{new:?}");
    }
}

#[test]
fn a_rect_leaf_is_updated_when_its_style_changes() {
    assert_eq!(
        kept(&rect(ComputedStyle::default()), &rect(wide())),
        Some(false)
    );
}

#[test]
fn an_image_leaf_is_updated_when_its_pixel_size_or_style_changes() {
    let old = image(Size::new(2, 2), ComputedStyle::default());
    assert_eq!(
        kept(&old, &image(Size::new(4, 2), ComputedStyle::default())),
        Some(false)
    );
    assert_eq!(kept(&old, &image(Size::new(2, 2), wide())), Some(false));
}

#[test]
fn a_leaf_of_another_kind_is_replaced() {
    assert_eq!(
        kept(
            &rect(ComputedStyle::default()),
            &text("a", ComputedStyle::default())
        ),
        None
    );
}
