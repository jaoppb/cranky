//! Shared measurer, layout roots, `StyledNode` builders and `RenderNode` accessors.
use crate::features::layout_engine::adapters::taffy::TaffyLayoutAdapter;
use crate::features::layout_engine::domain::{
    LayoutRoot, NodePath, RenderNode, StyledNode, SurfaceSpace, TextContent, TextMeasurer,
};
use crate::features::layout_engine::ports::LayoutEnginePort;
use crate::features::styling::domain::ComputedStyle;
use crate::features::vdom::domain::NodeKey;
use crate::shared::primitives::geometry::{Position, Size};
use crate::shared::primitives::{ModuleKey, ModuleName, ModuleOptions, SizeConstraint};
use crate::shared::rendering::ports::text::TextFont;

/// `node` laid out from the origin with no axis pinned.
pub(super) fn unpinned(node: StyledNode) -> LayoutRoot {
    LayoutRoot::new(node, Position::new(0, 0), SizeConstraint::none())
}

/// `node` laid out from the origin with its width pinned to `width` and its height free.
pub(super) fn pinned_width(node: StyledNode, width: u32) -> LayoutRoot {
    LayoutRoot::new(
        node,
        Position::new(0, 0),
        SizeConstraint::new(Some(width), None),
    )
}

/// Measures text at 10 pixels a byte, 20 high, and the `workspace` module at 150x28.
pub(super) struct MockMeasurer;

impl TextMeasurer for MockMeasurer {
    fn measure(&mut self, text: &str, _font: TextFont<'_>) -> Size {
        let len = u32::try_from(text.len()).unwrap_or(0);
        Size::new(len.saturating_mul(10), 20)
    }

    fn measure_module(&self, key: &ModuleKey) -> Option<Size> {
        if key.name() == "workspace" {
            Some(Size::new(150, 28))
        } else {
            None
        }
    }
}

/// Runs one layout pass of `root` with `MockMeasurer`, failing the test on an error.
pub(super) fn lay_out_root(adapter: &mut TaffyLayoutAdapter, root: LayoutRoot) -> RenderNode {
    adapter.calculate_layout(root, &mut MockMeasurer).unwrap()
}

/// Runs one unpinned layout pass of `node` with `MockMeasurer`.
pub(super) fn lay_out(adapter: &mut TaffyLayoutAdapter, node: StyledNode) -> RenderNode {
    lay_out_root(adapter, unpinned(node))
}

/// The bar's root path.
pub(super) fn root_path() -> NodePath {
    NodePath::root_in(SurfaceSpace::Bar)
}

/// The path of the root's child at `index`.
pub(super) fn child_path(index: usize) -> NodePath {
    NodePath::new(SurfaceSpace::Bar, vec![index])
}

/// An unkeyed text node with no actions.
pub(super) fn text_node(path: NodePath, text: &str, style: ComputedStyle) -> StyledNode {
    StyledNode::Text {
        path,
        node_key: None,
        text: TextContent::new(text.to_string()),
        style,
        on_click: None,
        on_hover: None,
        tooltip: None,
        popup: None,
        panel: None,
    }
}

/// A root text leaf with `style`.
pub(super) fn text_leaf(text: &str, style: ComputedStyle) -> StyledNode {
    text_node(root_path(), text, style)
}

/// A default-styled text child at `index`.
pub(super) fn text_child(index: usize, text: &str) -> StyledNode {
    text_node(child_path(index), text, ComputedStyle::default())
}

/// A default-styled text child at `index` keyed `key`.
pub(super) fn keyed_text_child(index: usize, key: &str, text: &str) -> StyledNode {
    StyledNode::Text {
        path: child_path(index),
        node_key: Some(NodeKey::new(key).unwrap()),
        text: TextContent::new(text.to_string()),
        style: ComputedStyle::default(),
        on_click: None,
        on_hover: None,
        tooltip: None,
        popup: None,
        panel: None,
    }
}

/// An embedded module node named `name` with default options.
pub(super) fn module_node(path: NodePath, name: &str, style: ComputedStyle) -> StyledNode {
    StyledNode::Module {
        path,
        node_key: None,
        key: ModuleKey::from_name(ModuleName::new(name)),
        options: ModuleOptions::default(),
        style,
        on_click: None,
        on_hover: None,
        tooltip: None,
        popup: None,
        panel: None,
    }
}

/// An unkeyed flex container holding `children`.
pub(super) fn flex_node(
    path: NodePath,
    children: Vec<StyledNode>,
    style: ComputedStyle,
) -> StyledNode {
    StyledNode::Flex {
        path,
        node_key: None,
        children,
        style,
        on_click: None,
        on_hover: None,
        tooltip: None,
        popup: None,
        panel: None,
    }
}

/// A default-styled root flex container holding `children`.
pub(super) fn container(children: Vec<StyledNode>) -> StyledNode {
    flex_node(root_path(), children, ComputedStyle::default())
}

/// An unkeyed root grid container holding `children`.
pub(super) fn grid_node(children: Vec<StyledNode>, style: ComputedStyle) -> StyledNode {
    StyledNode::Grid {
        path: root_path(),
        node_key: None,
        children,
        style,
        on_click: None,
        on_hover: None,
        tooltip: None,
        popup: None,
        panel: None,
    }
}

/// An unkeyed rect node with no actions.
pub(super) fn rect_node(path: NodePath, style: ComputedStyle) -> StyledNode {
    StyledNode::Rect {
        path,
        node_key: None,
        style,
        on_click: None,
        on_hover: None,
        tooltip: None,
        popup: None,
        panel: None,
    }
}

/// The children of a laid-out flex container; panics on any other node.
pub(super) fn flex_children(tree: RenderNode) -> Vec<RenderNode> {
    let RenderNode::Flex { children, .. } = tree else {
        panic!("Expected RenderNode::Flex");
    };
    children
}

/// The children of a laid-out grid container; panics on any other node.
pub(super) fn grid_children(tree: RenderNode) -> Vec<RenderNode> {
    let RenderNode::Grid { children, .. } = tree else {
        panic!("Expected RenderNode::Grid");
    };
    children
}

/// The text of a laid-out text leaf; panics on any other node.
pub(super) fn text_of(node: &RenderNode) -> &str {
    let RenderNode::Text { text, .. } = node else {
        panic!("Expected RenderNode::Text");
    };
    text.as_str()
}
