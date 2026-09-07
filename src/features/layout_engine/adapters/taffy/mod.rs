#![allow(unsafe_code)]

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
    #[allow(clippy::as_conversions, clippy::cast_precision_loss)]
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
            taffy::geometry::Size {
                width: taffy::style::AvailableSpace::Definite(size.width() as f32),
                height: taffy::style::AvailableSpace::Definite(size.height() as f32),
            }
        });

        // Compute layout
        self.taffy
            .compute_layout(root_node_id, available_space)
            .map_err(|e| LayoutError::EngineError(e.to_string()))?;

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
}
