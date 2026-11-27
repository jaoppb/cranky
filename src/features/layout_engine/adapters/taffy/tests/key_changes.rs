//! Keyed children: a key the last layout did not have is created in its place.
use super::super::TaffyLayoutAdapter;
use crate::features::layout_engine::domain::{
    LayoutRoot, NodePath, RenderNode, StyledNode, SurfaceSpace, TextContent, TextMeasurer,
};
use crate::features::layout_engine::ports::LayoutEnginePort;
use crate::features::styling::domain::ComputedStyle;
use crate::features::vdom::domain::NodeKey;
use crate::shared::primitives::geometry::{Position, Size};
use crate::shared::primitives::{ModuleKey, SizeConstraint};
use crate::shared::rendering::ports::text::TextFont;

/// Measures text at 10 pixels a byte, 20 high.
struct ByteWidths;

impl TextMeasurer for ByteWidths {
    fn measure(&mut self, text: &str, _font: TextFont<'_>) -> Size {
        Size::new(u32::try_from(text.len()).unwrap().saturating_mul(10), 20)
    }

    fn measure_module(&self, _key: &ModuleKey) -> Option<Size> {
        None
    }
}

/// A text child at `index`, keyed `key` when there is one.
fn text(index: usize, key: Option<&str>, content: &str) -> StyledNode {
    StyledNode::Text {
        path: NodePath::new(SurfaceSpace::Bar, vec![index]),
        node_key: key.map(|name| NodeKey::new(name).unwrap()),
        text: TextContent::new(content.to_string()),
        style: ComputedStyle::default(),
        on_click: None,
        on_hover: None,
        tooltip: None,
        popup: None,
        panel: None,
    }
}

fn row(children: Vec<StyledNode>) -> LayoutRoot {
    let node = StyledNode::Flex {
        path: NodePath::root_in(SurfaceSpace::Bar),
        node_key: None,
        children,
        style: ComputedStyle::default(),
        on_click: None,
        on_hover: None,
        tooltip: None,
        popup: None,
        panel: None,
    };
    LayoutRoot::new(node, Position::new(0, 0), SizeConstraint::none())
}

/// The left edge and width of each child of `tree`.
fn spans(tree: &RenderNode) -> Vec<(i32, u32)> {
    let boxes = tree.children().iter().map(RenderNode::rect);
    boxes.map(|rect| (rect.x(), rect.width())).collect()
}

#[test]
fn a_new_key_between_kept_ones_is_laid_out_in_its_place() {
    let mut adapter = TaffyLayoutAdapter::new();
    let first = vec![text(0, Some("a"), "a"), text(1, Some("b"), "bbb")];
    adapter
        .calculate_layout(row(first), &mut ByteWidths)
        .unwrap();

    let second = vec![
        text(0, Some("a"), "a"),
        text(1, Some("new"), "nn"),
        text(2, Some("b"), "bbb"),
    ];
    let tree = adapter
        .calculate_layout(row(second), &mut ByteWidths)
        .unwrap();
    assert_eq!(spans(&tree), [(0, 10), (10, 20), (30, 30)]);
}
