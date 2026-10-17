#![allow(unsafe_code)]

pub mod calc_resolve;
pub mod diff;
pub mod grid_converter;
mod leaf_size;
pub mod reconciler;
pub mod render_tree_builder;
pub mod style_builder;
pub mod style_converter;
pub mod tree_builder;

use crate::features::layout_engine::domain::{LayoutError, RenderNode, StyledNode, TextMeasurer};
use crate::features::layout_engine::ports::LayoutEnginePort;
use crate::shared::primitives::SizeConstraint;
use crate::shared::primitives::geometry::{Position, Size};
use crate::utils::f32_to_u32;
use calc_resolve::resolve_pending_calcs;
use diff::diff;
use reconciler::{LayoutState, apply_patch, build_layout_state};
use render_tree_builder::build_render_tree;
use taffy::style::{Dimension, Style};
use taffy::tree::NodeId;
use taffy::TaffyTree;
use tree_builder::TaffyTreeBuilder;

/// A pinned axis becomes a definite available space; an unpinned axis is
/// `MaxContent` — "as much room as the content wants" — which is exactly
/// what an entirely unconstrained layout already used on both axes, so a
/// fully-`None` `SizeConstraint` reproduces today's intrinsic-everywhere
/// behaviour unchanged.
fn axis_available_space(pinned: Option<u32>) -> taffy::style::AvailableSpace {
    pinned.map_or(taffy::style::AvailableSpace::MaxContent, |px| {
        taffy::style::AvailableSpace::Definite(f32::from(u16::try_from(px).unwrap_or(u16::MAX)))
    })
}

pub struct TaffyLayoutAdapter {
    taffy: TaffyTree,
    state: Option<LayoutState>,
    /// Root size of the last pass's unpinned layout; `None` if it wasn't pinned.
    intrinsic: Option<Size>,
}

#[allow(clippy::non_send_fields_in_send_ty)]
unsafe impl Send for TaffyLayoutAdapter {}
unsafe impl Sync for TaffyLayoutAdapter {}

impl Default for TaffyLayoutAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl TaffyLayoutAdapter {
    #[must_use]
    pub fn new() -> Self {
        Self {
            taffy: TaffyTree::new(),
            state: None,
            intrinsic: None,
        }
    }

    /// Drops the cached tree and layout state so the next pass rebuilds from
    /// scratch.
    ///
    /// Every fallible step of a layout pass runs *after* `apply_patch` has
    /// already mutated the tree, so an early return would leave `self.state`
    /// naming nodes that were freed. Taffy panics on a freed `NodeId` instead
    /// of returning an error, so a half-applied pass has to be thrown away
    /// whole.
    fn reset(&mut self) {
        self.taffy = TaffyTree::new();
        self.state = None;
    }

    /// Pins the root's size to `constraint` and returns the style to
    /// restore afterwards. A pinned axis is the embedding module's host box,
    /// which the child fills (the Shadow DOM split): the root takes the
    /// pinned size itself, not merely the room to grow into, so `1fr`
    /// tracks and `space-between` have a definite box to distribute.
    fn pin_root(
        &mut self,
        root: NodeId,
        constraint: SizeConstraint,
    ) -> Result<Style, LayoutError> {
        let engine_err = |e: taffy::TaffyError| LayoutError::EngineError(e.to_string());
        let original = self.taffy.style(root).map_err(engine_err)?.clone();
        let mut pinned = original.clone();
        let px = |v: u32| Dimension::length(f32::from(u16::try_from(v).unwrap_or(u16::MAX)));
        if let Some(w) = constraint.width() {
            pinned.size.width = px(w);
        }
        if let Some(h) = constraint.height() {
            pinned.size.height = px(h);
        }
        self.taffy.set_style(root, pinned).map_err(engine_err)?;
        Ok(original)
    }

    fn layout_pass(
        &mut self,
        node: &StyledNode,
        measurer: &mut dyn TextMeasurer,
        start_pos: Position,
        constraint: SizeConstraint,
    ) -> Result<RenderNode, LayoutError> {
        let mut builder = TaffyTreeBuilder::new(&mut self.taffy);
        let new_state = if let Some(state) = &self.state {
            let patch = diff(state, node, measurer);
            apply_patch(&mut builder, patch, measurer)?
        } else {
            build_layout_state(&mut builder, node, measurer)?
        };

        let root_node_id = new_state.root_node;

        let available_space = taffy::geometry::Size {
            width: axis_available_space(constraint.width()),
            height: axis_available_space(constraint.height()),
        };

        // A pinned root is laid out twice: unpinned first, whose size is
        // what an embedder may measure it by, then pinned to the host box.
        self.intrinsic = None;
        let unpinned_root = if constraint.width().is_some() || constraint.height().is_some() {
            self.taffy
                .compute_layout(root_node_id, taffy::geometry::Size { width: axis_available_space(None), height: axis_available_space(None) })
                .map_err(|e| LayoutError::EngineError(e.to_string()))?;
            let size = self
                .taffy
                .layout(root_node_id)
                .map_err(|e| LayoutError::EngineError(e.to_string()))?
                .size;
            self.intrinsic = Some(Size::new(f32_to_u32(size.width), f32_to_u32(size.height)));
            Some(self.pin_root(root_node_id, constraint)?)
        } else {
            None
        };

        // Compute layout
        self.taffy
            .compute_layout(root_node_id, available_space)
            .map_err(|e| LayoutError::EngineError(e.to_string()))?;

        // A node styled with `calc(<percent>% + <px>px)` — e.g.
        // `width: calc(100% - 2rem)` — can't resolve its percent term
        // until its parent's size from this first pass is known. Patch any
        // such nodes with the now-concrete pixel value and lay out again;
        // ordinary trees (no calc()) never pay this cost.
        if resolve_pending_calcs(&mut self.taffy, &new_state) {
            self.taffy
                .compute_layout(root_node_id, available_space)
                .map_err(|e| LayoutError::EngineError(e.to_string()))?;
        }

        // Build RenderNode tree
        let render_tree = build_render_tree(&self.taffy, root_node_id, node, start_pos)?;

        // The diff only patches nodes whose style changed, so the root must
        // go back to its own style for the next pass to start clean.
        if let Some(original) = unpinned_root {
            self.taffy
                .set_style(root_node_id, original)
                .map_err(|e| LayoutError::EngineError(e.to_string()))?;
        }

        tracing::trace!(
            rect = ?render_tree.rect(),
            "Calculated layout render tree"
        );

        self.state = Some(new_state);

        Ok(render_tree)
    }
}

impl LayoutEnginePort for TaffyLayoutAdapter {
    fn calculate_layout_with_constraints(
        &mut self,
        node: StyledNode,
        measurer: &mut dyn TextMeasurer,
        start_pos: Position,
        constraint: SizeConstraint,
    ) -> Result<RenderNode, LayoutError> {
        let result = self.layout_pass(&node, measurer, start_pos, constraint);
        if result.is_err() {
            self.reset();
        }
        result
    }

    fn intrinsic_size(&self) -> Option<Size> {
        self.intrinsic
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::layout_engine::domain::{
        NodePath, StyledNode, SurfaceSpace, TextContent, TextMeasurer,
    };
    use crate::features::styling::domain::{ComputedStyle, GridTrack};
    use crate::features::vdom::domain::NodeKey;
    use crate::shared::config::domain::{FontFamily, FontSize};
    use crate::shared::primitives::geometry::{Position, Size};
    use crate::shared::primitives::{ModuleName, ModuleOptions};

    struct MockMeasurer;
    impl TextMeasurer for MockMeasurer {
        fn measure(
            &mut self,
            text: &str,
            _font: Option<&FontFamily>,
            _size: Option<FontSize>,
        ) -> Size {
            let len = u32::try_from(text.len()).unwrap_or(0);
            Size::new(len.saturating_mul(10), 20)
        }

        fn measure_module(&self, key: &crate::shared::primitives::ModuleKey) -> Option<Size> {
            if key.name() == "workspace" {
                Some(Size::new(150, 28))
            } else {
                None
            }
        }
    }

    #[test]
    fn test_calculate_layout_styled_text() {
        let mut adapter = TaffyLayoutAdapter::new();
        let mut measurer = MockMeasurer;

        let node = StyledNode::Text {
            path: NodePath::root_in(SurfaceSpace::Bar),
            node_key: None,
            text: TextContent::new("hello".to_string()),
            style: ComputedStyle::default(),
            on_click: None,
            on_hover: None,
            tooltip: None,
            popup: None,
            panel: None,
        };

        let render_tree = adapter
            .calculate_layout(node, &mut measurer, Position::new(0, 0))
            .unwrap();
        assert_eq!(render_tree.rect().width(), 50);
        assert_eq!(render_tree.rect().height(), 20);
    }

    #[test]
    fn test_size_constraint_pins_one_axis_and_leaves_the_other_intrinsic() {
        // A `width: 100%` root has nothing to resolve against on its own —
        // it needs a definite available space on that axis, which is
        // exactly what an embedded module's SizeConstraint supplies for a
        // pinned axis and withholds for a free one.
        let mut style = ComputedStyle::default();
        style.set_width(crate::features::styling::domain::CssLength::Percent(100.0));
        let node = || StyledNode::Text {
            path: NodePath::root_in(SurfaceSpace::Bar),
            node_key: None,
            text: TextContent::new("hello".to_string()),
            style: style.clone(),
            on_click: None,
            on_hover: None,
            tooltip: None,
            popup: None,
            panel: None,
        };

        // Width pinned to 300, height left intrinsic: width resolves the
        // 100% against the pin; height still comes from the measurer, not
        // from the (nonexistent) pin.
        let mut adapter = TaffyLayoutAdapter::new();
        let mut measurer = MockMeasurer;
        let pinned = adapter
            .calculate_layout_with_constraints(
                node(),
                &mut measurer,
                Position::new(0, 0),
                crate::shared::primitives::SizeConstraint::new(Some(300), None),
            )
            .unwrap();
        assert_eq!(pinned.rect().width(), 300);
        assert_eq!(pinned.rect().height(), 20);

        // No constraint at all: `100%` has no containing block to resolve
        // against (MaxContent isn't a definite basis), so it resolves to 0
        // — never to whatever a parent's rect happened to be. This is the
        // actual fix: under the old model `available_size` came from
        // `current_bounds`, so a `width: 100%` child always matched
        // whatever slot the parent last assigned, which is exactly the
        // feedback loop decision 10 removes. Now only an explicit
        // `SizeConstraint` pin can produce a definite available space.
        let mut adapter = TaffyLayoutAdapter::new();
        let unconstrained = adapter
            .calculate_layout(node(), &mut measurer, Position::new(0, 0))
            .unwrap();
        assert_eq!(unconstrained.rect().width(), 0);
        assert_eq!(unconstrained.rect().height(), 20);
    }

    #[test]
    fn test_calculate_layout_styled_module_with_custom_size() {
        let mut adapter = TaffyLayoutAdapter::new();
        let mut measurer = MockMeasurer;

        let mut style = ComputedStyle::default();
        style.set_width(crate::features::styling::domain::CssLength::Px(120.0));
        style.set_height(crate::features::styling::domain::CssLength::Px(30.0));

        let node = StyledNode::Module {
            path: NodePath::root_in(SurfaceSpace::Bar),
            node_key: None,
            key: crate::shared::primitives::ModuleKey::from_name(ModuleName::new("custom_mod")),
            options: ModuleOptions::default(),
            style,
            on_click: None,
            on_hover: None,
            tooltip: None,
            popup: None,
            panel: None,
        };

        let render_tree = adapter
            .calculate_layout(node, &mut measurer, Position::new(10, 5))
            .unwrap();
        assert_eq!(render_tree.rect().x(), 10);
        assert_eq!(render_tree.rect().y(), 5);
        assert_eq!(render_tree.rect().width(), 120);
        assert_eq!(render_tree.rect().height(), 30);
    }

    #[test]
    fn test_calculate_layout_styled_module_with_measured_size() {
        let mut adapter = TaffyLayoutAdapter::new();
        let mut measurer = MockMeasurer;

        let node = StyledNode::Module {
            path: NodePath::root_in(SurfaceSpace::Bar),
            node_key: None,
            key: crate::shared::primitives::ModuleKey::from_name(ModuleName::new("workspace")),
            options: ModuleOptions::default(),
            style: ComputedStyle::default(),
            on_click: None,
            on_hover: None,
            tooltip: None,
            popup: None,
            panel: None,
        };

        let render_tree = adapter
            .calculate_layout(node, &mut measurer, Position::new(0, 0))
            .unwrap();
        assert_eq!(render_tree.rect().width(), 150);
        assert_eq!(render_tree.rect().height(), 28);
    }

    #[test]
    fn test_nested_container_module_collect_layouts() {
        let mut adapter = TaffyLayoutAdapter::new();
        let mut measurer = MockMeasurer;

        let child1 = StyledNode::Module {
            path: NodePath::new(SurfaceSpace::Bar, vec![0]),
            node_key: None,
            key: crate::shared::primitives::ModuleKey::from_name(ModuleName::new("workspace")),
            options: ModuleOptions::default(),
            style: ComputedStyle::default(),
            on_click: None,
            on_hover: None,
            tooltip: None,
            popup: None,
            panel: None,
        };

        let mut root_style = ComputedStyle::default();
        root_style.set_padding(crate::features::layout_engine::domain::BoxMargin::new(
            0.0, 0.0, 16.0, 16.0,
        ));
        let root = StyledNode::Flex {
            path: NodePath::root_in(SurfaceSpace::Bar),
            node_key: None,
            children: vec![child1],
            style: root_style,
            on_click: None,
            on_hover: None,
            tooltip: None,
            popup: None,
            panel: None,
        };

        let render_tree = adapter
            .calculate_layout(root, &mut measurer, Position::new(0, 0))
            .unwrap();

        let layouts = render_tree.collect_module_layouts();
        assert_eq!(layouts.len(), 1);
        assert_eq!(layouts[0].key().name().as_str(), "workspace");
        assert_eq!(layouts[0].bounds().x(), 16);
    }

    #[test]
    fn test_calculate_layout_grid() {
        let mut adapter = TaffyLayoutAdapter::new();
        let mut measurer = MockMeasurer;

        let mut child1_style = ComputedStyle::default();
        child1_style.set_width(crate::features::styling::domain::CssLength::Percent(100.0));
        let child1 = StyledNode::Rect {
            path: NodePath::new(SurfaceSpace::Bar, vec![0]),
            node_key: None,
            style: child1_style,
            on_click: None,
            on_hover: None,
            tooltip: None,
            popup: None,
            panel: None,
        };

        let mut child2_style = ComputedStyle::default();
        child2_style.set_width(crate::features::styling::domain::CssLength::Percent(100.0));
        let child2 = StyledNode::Rect {
            path: NodePath::new(SurfaceSpace::Bar, vec![1]),
            node_key: None,
            style: child2_style,
            on_click: None,
            on_hover: None,
            tooltip: None,
            popup: None,
            panel: None,
        };

        let mut grid_style = ComputedStyle::default_for_grid();
        grid_style.set_grid_template_columns(vec![GridTrack::Px(50.0), GridTrack::Px(100.0)]);
        grid_style.set_column_gap(crate::features::layout_engine::domain::Gap::new(10.0));

        let grid = StyledNode::Grid {
            path: NodePath::root_in(SurfaceSpace::Bar),
            node_key: None,
            children: vec![child1, child2],
            style: grid_style,
            on_click: None,
            on_hover: None,
            tooltip: None,
            popup: None,
            panel: None,
        };

        let render_tree = adapter
            .calculate_layout(grid, &mut measurer, Position::new(0, 0))
            .unwrap();

        if let RenderNode::Grid { children, .. } = render_tree {
            assert_eq!(children.len(), 2);
            assert_eq!(children[0].rect().x(), 0);
            assert_eq!(children[0].rect().width(), 50);
            assert_eq!(children[1].rect().x(), 60); // 50 + 10 gap
            assert_eq!(children[1].rect().width(), 100);
        } else {
            panic!("Expected RenderNode::Grid");
        }
    }

    #[test]
    fn test_calc_percent_resolves_against_real_parent_width_second_pass() {
        // width: calc(50% - 20px) on a child of a 200px-wide parent — this
        // can only resolve correctly once the parent's actual layout size
        // is known, proving the two-pass mechanism (not just the style
        // cascade) actually runs.
        let mut adapter = TaffyLayoutAdapter::new();
        let mut measurer = MockMeasurer;

        let mut child_style = ComputedStyle::default();
        child_style.set_width(crate::features::styling::domain::CssLength::Calc {
            percent: 50.0,
            px: -20.0,
        });
        child_style.set_height(crate::features::styling::domain::CssLength::Px(10.0));
        let child = StyledNode::Rect {
            path: NodePath::new(SurfaceSpace::Bar, vec![0]),
            node_key: None,
            style: child_style,
            on_click: None,
            on_hover: None,
            tooltip: None,
            popup: None,
            panel: None,
        };

        let mut root_style = ComputedStyle::default();
        root_style.set_width(crate::features::styling::domain::CssLength::Px(200.0));
        root_style.set_height(crate::features::styling::domain::CssLength::Px(10.0));
        let root = StyledNode::Flex {
            path: NodePath::root_in(SurfaceSpace::Bar),
            node_key: None,
            children: vec![child],
            style: root_style,
            on_click: None,
            on_hover: None,
            tooltip: None,
            popup: None,
            panel: None,
        };

        let render_tree = adapter
            .calculate_layout(root, &mut measurer, Position::new(0, 0))
            .unwrap();

        if let RenderNode::Flex { children, .. } = render_tree {
            // 200px * 50% - 20px = 80px.
            assert_eq!(children[0].rect().width(), 80);
        } else {
            panic!("Expected RenderNode::Flex");
        }
    }
    fn text_child(index: usize, text: &str) -> StyledNode {
        StyledNode::Text {
            path: NodePath::new(SurfaceSpace::Bar, vec![index]),
            node_key: None,
            text: TextContent::new(text.to_string()),
            style: ComputedStyle::default(),
            on_click: None,
            on_hover: None,
            tooltip: None,
            popup: None,
            panel: None,
        }
    }

    fn container(children: Vec<StyledNode>) -> StyledNode {
        StyledNode::Flex {
            path: NodePath::root_in(SurfaceSpace::Bar),
            node_key: None,
            children,
            style: ComputedStyle::default(),
            on_click: None,
            on_hover: None,
            tooltip: None,
            popup: None,
            panel: None,
        }
    }

    /// A container that empties and then repopulates must not resurrect the
    /// taffy nodes it freed on the way down.
    ///
    /// `workspace.lua` renders exactly this shape: an unconditional
    /// `ui.flex` whose children vanish when the monitor it is rendering for
    /// owns no workspaces. Before the fix, the emptying pass freed every
    /// child node but stored the freed `LayoutState`s back into `self.state`,
    /// and the following pass handed one of those dead ids to
    /// `remove_recursive` -> `TaffyTree::children` -> `SlotMap` index panic
    /// ("invalid `SlotMap` key used").
    #[test]
    fn test_layout_children_empty_then_repopulated() {
        let mut adapter = TaffyLayoutAdapter::new();
        let mut measurer = MockMeasurer;

        let populated = container(vec![
            text_child(0, "one"),
            text_child(1, "two"),
            text_child(2, "three"),
        ]);
        adapter
            .calculate_layout(populated, &mut measurer, Position::new(0, 0))
            .unwrap();

        // N -> 0: every child node is freed.
        adapter
            .calculate_layout(container(vec![]), &mut measurer, Position::new(0, 0))
            .unwrap();

        // Still empty: the pass that used to panic on a freed NodeId.
        adapter
            .calculate_layout(container(vec![]), &mut measurer, Position::new(0, 0))
            .unwrap();

        // 0 -> N: the freed children must not be diffed against.
        let repopulated = container(vec![text_child(0, "one"), text_child(1, "two")]);
        let render_tree = adapter
            .calculate_layout(repopulated, &mut measurer, Position::new(0, 0))
            .unwrap();

        let RenderNode::Flex { children, .. } = render_tree else {
            panic!("Expected RenderNode::Flex");
        };
        assert_eq!(children.len(), 2);
        assert_eq!(children[0].rect().width(), 30);
        assert_eq!(children[1].rect().width(), 30);
    }

    /// The same transition one level down, where the emptying container is
    /// itself a child being patched rather than the diff root.
    #[test]
    fn test_layout_nested_children_empty_then_repopulated() {
        let mut adapter = TaffyLayoutAdapter::new();
        let mut measurer = MockMeasurer;

        let nested = |texts: Vec<&str>| {
            let inner = StyledNode::Flex {
                path: NodePath::new(SurfaceSpace::Bar, vec![0]),
                node_key: None,
                children: texts
                    .into_iter()
                    .enumerate()
                    .map(|(i, t)| text_child(i, t))
                    .collect(),
                style: ComputedStyle::default(),
                on_click: None,
                on_hover: None,
                tooltip: None,
                popup: None,
                panel: None,
            };
            container(vec![inner])
        };

        for frame in [
            nested(vec!["a", "b"]),
            nested(vec![]),
            nested(vec![]),
            nested(vec!["c"]),
        ] {
            adapter
                .calculate_layout(frame, &mut measurer, Position::new(0, 0))
                .unwrap();
        }
    }

    fn keyed_text_child(index: usize, key: &str, text: &str) -> StyledNode {
        StyledNode::Text {
            path: NodePath::new(SurfaceSpace::Bar, vec![index]),
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

    fn keyed_container(children: Vec<StyledNode>) -> StyledNode {
        StyledNode::Flex {
            path: NodePath::root_in(SurfaceSpace::Bar),
            node_key: None,
            children,
            style: ComputedStyle::default(),
            on_click: None,
            on_hover: None,
            tooltip: None,
            popup: None,
            panel: None,
        }
    }

    /// Removing a middle keyed child must not disturb the taffy nodes of the
    /// siblings that come after it. Positional matching would diff
    /// `"b"`'s old state against `"c"`'s new content (since `"c"` slides
    /// into index 1) and free whatever used to be at the trailing index —
    /// churn a keyed list exists specifically to avoid.
    #[test]
    fn test_layout_keyed_children_removes_middle_without_touching_siblings() {
        let mut adapter = TaffyLayoutAdapter::new();
        let mut measurer = MockMeasurer;

        let first = keyed_container(vec![
            keyed_text_child(0, "a", "a-content"),
            keyed_text_child(1, "b", "b-content"),
            keyed_text_child(2, "c", "c-content"),
        ]);
        let first_tree = adapter
            .calculate_layout(first, &mut measurer, Position::new(0, 0))
            .unwrap();
        let RenderNode::Flex {
            children: first_children,
            ..
        } = first_tree
        else {
            panic!("Expected RenderNode::Flex");
        };
        assert_eq!(first_children.len(), 3);

        // Remove "b" — "a" and "c" keep their keys and content.
        let second = keyed_container(vec![
            keyed_text_child(0, "a", "a-content"),
            keyed_text_child(1, "c", "c-content"),
        ]);
        let second_tree = adapter
            .calculate_layout(second, &mut measurer, Position::new(0, 0))
            .unwrap();
        let RenderNode::Flex {
            children: second_children,
            ..
        } = second_tree
        else {
            panic!("Expected RenderNode::Flex");
        };
        assert_eq!(second_children.len(), 2);
        // Positions come from the new list's order, matched by key rather
        // than by the old index each survivor used to occupy.
        let RenderNode::Text { text: t0, .. } = &second_children[0] else {
            panic!("Expected RenderNode::Text");
        };
        let RenderNode::Text { text: t1, .. } = &second_children[1] else {
            panic!("Expected RenderNode::Text");
        };
        assert_eq!(t0.as_str(), "a-content");
        assert_eq!(t1.as_str(), "c-content");
    }

    /// A container reordering its keyed children (no insertions or
    /// removals) must reorder rather than replace: same identities, new
    /// order.
    #[test]
    fn test_layout_keyed_children_reorder() {
        let mut adapter = TaffyLayoutAdapter::new();
        let mut measurer = MockMeasurer;

        let first = keyed_container(vec![
            keyed_text_child(0, "a", "a-content"),
            keyed_text_child(1, "b", "b-content"),
            keyed_text_child(2, "c", "c-content"),
        ]);
        adapter
            .calculate_layout(first, &mut measurer, Position::new(0, 0))
            .unwrap();

        let reordered = keyed_container(vec![
            keyed_text_child(0, "c", "c-content"),
            keyed_text_child(1, "a", "a-content"),
            keyed_text_child(2, "b", "b-content"),
        ]);
        let render_tree = adapter
            .calculate_layout(reordered, &mut measurer, Position::new(0, 0))
            .unwrap();

        let RenderNode::Flex { children, .. } = render_tree else {
            panic!("Expected RenderNode::Flex");
        };
        assert_eq!(children.len(), 3);
        let texts: Vec<&str> = children
            .iter()
            .map(|c| {
                let RenderNode::Text { text, .. } = c else {
                    panic!("Expected RenderNode::Text");
                };
                text.as_str()
            })
            .collect();
        assert_eq!(texts, vec!["c-content", "a-content", "b-content"]);
    }

    /// A list where only some children carry a key (or where a key repeats)
    /// has no well-defined keyed match, so it must fall back to positional
    /// reconciliation rather than panicking or mismatching identities.
    #[test]
    fn test_layout_mixed_keys_falls_back_to_positional_without_panicking() {
        let mut adapter = TaffyLayoutAdapter::new();
        let mut measurer = MockMeasurer;

        let mixed_first = keyed_container(vec![
            keyed_text_child(0, "a", "a-content"),
            text_child(1, "unkeyed"),
        ]);
        adapter
            .calculate_layout(mixed_first, &mut measurer, Position::new(0, 0))
            .unwrap();

        let mixed_second = keyed_container(vec![text_child(0, "still-unkeyed")]);
        let render_tree = adapter
            .calculate_layout(mixed_second, &mut measurer, Position::new(0, 0))
            .unwrap();

        let RenderNode::Flex { children, .. } = render_tree else {
            panic!("Expected RenderNode::Flex");
        };
        assert_eq!(children.len(), 1);

        // Duplicate keys: same guard, exercised independently of the mixed
        // case above.
        let dup_first = keyed_container(vec![
            keyed_text_child(0, "dup", "one"),
            keyed_text_child(1, "dup", "two"),
        ]);
        adapter
            .calculate_layout(dup_first, &mut measurer, Position::new(0, 0))
            .unwrap();
        let dup_second = keyed_container(vec![keyed_text_child(0, "dup", "one")]);
        adapter
            .calculate_layout(dup_second, &mut measurer, Position::new(0, 0))
            .unwrap();
    }

    #[test]
    fn test_dropping_a_constraint_unpins_the_cached_root() {
        let grid = || {
            let mut style = ComputedStyle::default_for_grid();
            style.set_grid_template_columns(vec![GridTrack::Fr(1.0)]);
            StyledNode::Grid {
                path: NodePath::root_in(SurfaceSpace::Bar),
                node_key: None,
                children: vec![text_leaf("ab", ComputedStyle::default())],
                style,
                on_click: None,
                on_hover: None,
                tooltip: None,
                popup: None,
                panel: None,
            }
        };
        let mut adapter = TaffyLayoutAdapter::new();
        let mut measurer = MockMeasurer;
        let pinned = adapter
            .calculate_layout_with_constraints(
                grid(),
                &mut measurer,
                Position::new(0, 0),
                SizeConstraint::new(Some(280), None),
            )
            .unwrap();
        assert_eq!(pinned.rect().width(), 280);
        // What the embedder measures by is the unpinned size, not the pin.
        assert_eq!(adapter.intrinsic_size(), Some(Size::new(20, 20)));

        let free = adapter
            .calculate_layout(grid(), &mut measurer, Position::new(0, 0))
            .unwrap();
        assert_eq!(free.rect().width(), 20);
        assert_eq!(adapter.intrinsic_size(), None);
    }

    fn text_leaf(text: &str, style: ComputedStyle) -> StyledNode {
        StyledNode::Text {
            path: NodePath::root_in(SurfaceSpace::Bar),
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

    #[test]
    fn test_leaf_padding_grows_the_box_instead_of_eating_the_glyph() {
        // Issue #9: `.day { padding: 4px 2px }` hugged the glyph because the
        // leaf's size was the bare glyph size, so border-box padding came
        // out of it. 5 glyphs * 10 + 2px each side, 20 + 4px each end.
        let mut style = ComputedStyle::default();
        style.set_padding(crate::features::layout_engine::domain::BoxMargin::new(
            4.0, 4.0, 2.0, 2.0,
        ));
        let mut adapter = TaffyLayoutAdapter::new();
        let mut measurer = MockMeasurer;

        let tree = adapter
            .calculate_layout(text_leaf("hello", style), &mut measurer, Position::new(0, 0))
            .unwrap();

        assert_eq!(tree.rect().width(), 54);
        assert_eq!(tree.rect().height(), 28);
    }

    #[test]
    fn test_fr_columns_fill_a_pinned_root_width() {
        // Issue #9: `repeat(7, 1fr)` collapsed to content-sized columns
        // because the child was never given a definite width. Pinned to
        // 280, seven `1fr` tracks must split it evenly (40px each).
        let cell = |i: usize| StyledNode::Text {
            path: NodePath::new(SurfaceSpace::Bar, vec![i]),
            node_key: None,
            text: TextContent::new("1".to_string()),
            style: ComputedStyle::default(),
            on_click: None,
            on_hover: None,
            tooltip: None,
            popup: None,
            panel: None,
        };
        let mut grid_style = ComputedStyle::default_for_grid();
        grid_style.set_grid_template_columns(vec![GridTrack::Fr(1.0); 7]);
        let grid = StyledNode::Grid {
            path: NodePath::root_in(SurfaceSpace::Bar),
            node_key: None,
            children: (0..7).map(cell).collect(),
            style: grid_style,
            on_click: None,
            on_hover: None,
            tooltip: None,
            popup: None,
            panel: None,
        };
        let mut adapter = TaffyLayoutAdapter::new();
        let mut measurer = MockMeasurer;

        let tree = adapter
            .calculate_layout_with_constraints(
                grid,
                &mut measurer,
                Position::new(0, 0),
                crate::shared::primitives::SizeConstraint::new(Some(280), None),
            )
            .unwrap();

        assert_eq!(tree.rect().width(), 280);
        let RenderNode::Grid { children, .. } = tree else {
            panic!("Expected RenderNode::Grid");
        };
        let xs: Vec<u32> = children.iter().map(|c| u32::try_from(c.rect().x()).unwrap()).collect();
        assert_eq!(xs, vec![0, 40, 80, 120, 160, 200, 240]);
    }

    #[test]
    fn test_embedder_min_width_overrides_the_childs_reported_width() {
        // Issue #9 decision: a rule on the embed node (`.calendar {
        // min-width: 280px }`) wins over the child's own default (reported
        // as 240) and persists — it is a real constraint, not a hint.
        struct CalendarMeasurer;
        impl TextMeasurer for CalendarMeasurer {
            fn measure(&mut self, _: &str, _: Option<&FontFamily>, _: Option<FontSize>) -> Size {
                Size::new(0, 0)
            }
            fn measure_module(&self, _: &crate::shared::primitives::ModuleKey) -> Option<Size> {
                Some(Size::new(240, 190))
            }
        }
        let mut style = ComputedStyle::default();
        style.set_min_width(crate::features::styling::domain::CssLength::Px(280.0));
        let node = StyledNode::Module {
            path: NodePath::root_in(SurfaceSpace::Bar),
            node_key: None,
            key: crate::shared::primitives::ModuleKey::from_name(ModuleName::new("calendar")),
            options: ModuleOptions::default(),
            style,
            on_click: None,
            on_hover: None,
            tooltip: None,
            popup: None,
            panel: None,
        };
        let mut adapter = TaffyLayoutAdapter::new();

        let tree = adapter
            .calculate_layout(node, &mut CalendarMeasurer, Position::new(0, 0))
            .unwrap();

        assert_eq!(tree.rect().width(), 280);
        assert_eq!(tree.rect().height(), 190);
    }
}
