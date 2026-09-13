#![allow(unsafe_code)]

pub mod calc_resolve;
pub mod diff;
pub mod grid_converter;
pub mod reconciler;
pub mod render_tree_builder;
pub mod style_builder;
pub mod style_converter;
pub mod tree_builder;

use crate::features::layout_engine::domain::{LayoutError, RenderNode, StyledNode, TextMeasurer};
use crate::features::layout_engine::ports::LayoutEnginePort;
use crate::shared::primitives::geometry::{Position, Size};
use calc_resolve::resolve_pending_calcs;
use diff::diff;
use reconciler::{apply_patch, build_layout_state, LayoutState};
use render_tree_builder::build_render_tree;
use taffy::prelude::TaffyMaxContent;
use taffy::TaffyTree;
use tree_builder::TaffyTreeBuilder;

pub struct TaffyLayoutAdapter {
    taffy: TaffyTree,
    state: Option<LayoutState>,
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
        }
    }
}

impl LayoutEnginePort for TaffyLayoutAdapter {
    fn calculate_layout_with_constraints(
        &mut self,
        node: StyledNode,
        measurer: &mut dyn TextMeasurer,
        start_pos: Position,
        available_size: Option<Size>,
    ) -> Result<RenderNode, LayoutError> {
        let mut builder = TaffyTreeBuilder::new(&mut self.taffy);
        let new_state = if let Some(state) = &self.state {
            let patch = diff(state, &node, measurer);
            apply_patch(&mut builder, patch, measurer)?
        } else {
            build_layout_state(&mut builder, &node, measurer)?
        };

        let root_node_id = new_state.root_node;

        let available_space = available_size.map_or(taffy::geometry::Size::MAX_CONTENT, |size| {
            let w = f32::from(u16::try_from(size.width()).unwrap_or(u16::MAX));
            let h = f32::from(u16::try_from(size.height()).unwrap_or(u16::MAX));
            taffy::geometry::Size {
                width: taffy::style::AvailableSpace::Definite(w),
                height: taffy::style::AvailableSpace::Definite(h),
            }
        });

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
        let render_tree = build_render_tree(&self.taffy, root_node_id, &node, start_pos)?;

        tracing::trace!(
            rect = ?render_tree.rect(),
            "Calculated layout render tree"
        );

        self.state = Some(new_state);

        Ok(render_tree)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::layout_engine::domain::{NodePath, StyledNode, TextContent, TextMeasurer};
    use crate::features::styling::domain::{ComputedStyle, GridTrack};
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
            path: NodePath::root(),
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
    fn test_calculate_layout_styled_module_with_custom_size() {
        let mut adapter = TaffyLayoutAdapter::new();
        let mut measurer = MockMeasurer;

        let mut style = ComputedStyle::default();
        style.set_width(crate::features::styling::domain::CssLength::Px(120.0));
        style.set_height(crate::features::styling::domain::CssLength::Px(30.0));

        let node = StyledNode::Module {
            path: NodePath::root(),
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
            path: NodePath::root(),
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
            path: NodePath::new(vec![0]),
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
            path: NodePath::root(),
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
            path: NodePath::new(vec![0]),
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
            path: NodePath::new(vec![1]),
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
            path: NodePath::root(),
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
            path: NodePath::new(vec![0]),
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
            path: NodePath::root(),
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
}
