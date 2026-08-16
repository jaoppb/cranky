use crate::features::layout_engine::domain::LayoutError;
use crate::features::layout_engine::domain::{
    AlignItems, FlexDirection, JustifyContent, LayoutNode, RenderNode, TextMeasurer,
};
use crate::shared::primitives::geometry::{Position, Rect, Size};
use crate::features::layout_engine::ports::LayoutEnginePort;
use taffy::prelude::TaffyMaxContent;
use taffy::{
    TaffyTree,
    geometry::Size as TaffySize,
    style::Dimension,
    style::LengthPercentage,
    style::{
        AlignItems as TaffyAlignItems, FlexDirection as TaffyFlexDirection,
        JustifyContent as TaffyJustifyContent, Style,
    },
    tree::NodeId,
};

#[derive(Clone)]
struct LayoutState {
    root_node: taffy::prelude::NodeId,
    layout: crate::features::layout_engine::domain::LayoutNode,
    children: Vec<LayoutState>,
}

enum Patch<'a> {
    Keep(&'a LayoutState),
    Update {
        old_state: &'a LayoutState,
        new_layout: &'a crate::features::layout_engine::domain::LayoutNode,
        style: Box<Option<Style>>,
        children: Option<Vec<Patch<'a>>>,
    },
    Replace {
        old_state: &'a LayoutState,
        new_layout: &'a crate::features::layout_engine::domain::LayoutNode,
    },
    Create {
        new_layout: &'a crate::features::layout_engine::domain::LayoutNode,
    },
}

pub struct TaffyLayoutAdapter {
    taffy: TaffyTree,
    state: Option<LayoutState>,
}

impl Default for TaffyLayoutAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl TaffyLayoutAdapter {
    pub fn new() -> Self {
        Self {
            taffy: TaffyTree::new(),
            state: None,
        }
    }
}

impl From<FlexDirection> for TaffyFlexDirection {
    fn from(dir: FlexDirection) -> Self {
        match dir {
            FlexDirection::Row => TaffyFlexDirection::Row,
            FlexDirection::Column => TaffyFlexDirection::Column,
        }
    }
}

impl From<JustifyContent> for TaffyJustifyContent {
    fn from(jc: JustifyContent) -> Self {
        match jc {
            JustifyContent::Start => TaffyJustifyContent::FLEX_START,
            JustifyContent::End => TaffyJustifyContent::FLEX_END,
            JustifyContent::Center => TaffyJustifyContent::CENTER,
            JustifyContent::SpaceBetween => TaffyJustifyContent::SPACE_BETWEEN,
            JustifyContent::SpaceAround => TaffyJustifyContent::SPACE_AROUND,
            JustifyContent::SpaceEvenly => TaffyJustifyContent::SPACE_EVENLY,
        }
    }
}

impl From<AlignItems> for TaffyAlignItems {
    fn from(ai: AlignItems) -> Self {
        match ai {
            AlignItems::Start => TaffyAlignItems::FLEX_START,
            AlignItems::End => TaffyAlignItems::FLEX_END,
            AlignItems::Center => TaffyAlignItems::CENTER,
            AlignItems::Stretch => TaffyAlignItems::STRETCH,
        }
    }
}

impl From<crate::features::layout_engine::domain::PositionType> for taffy::style::Position {
    fn from(pt: crate::features::layout_engine::domain::PositionType) -> Self {
        match pt {
            crate::features::layout_engine::domain::PositionType::Relative => taffy::style::Position::Relative,
            crate::features::layout_engine::domain::PositionType::Absolute => taffy::style::Position::Absolute,
        }
    }
}

impl From<&crate::features::layout_engine::domain::BoxMargin> for taffy::geometry::Rect<LengthPercentage> {
    fn from(padding: &crate::features::layout_engine::domain::BoxMargin) -> Self {
        taffy::geometry::Rect {
            left: LengthPercentage::length(padding.left() as f32),
            right: LengthPercentage::length(padding.right() as f32),
            top: LengthPercentage::length(padding.top() as f32),
            bottom: LengthPercentage::length(padding.bottom() as f32),
        }
    }
}

impl From<&crate::features::layout_engine::domain::BoxMargin>
    for taffy::geometry::Rect<taffy::style::LengthPercentageAuto>
{
    fn from(margin: &crate::features::layout_engine::domain::BoxMargin) -> Self {
        taffy::geometry::Rect {
            left: LengthPercentage::length(margin.left() as f32).into(),
            right: LengthPercentage::length(margin.right() as f32).into(),
            top: LengthPercentage::length(margin.top() as f32).into(),
            bottom: LengthPercentage::length(margin.bottom() as f32).into(),
        }
    }
}

impl From<&crate::features::layout_engine::domain::Gap> for TaffySize<LengthPercentage> {
    fn from(gap: &crate::features::layout_engine::domain::Gap) -> Self {
        TaffySize {
            width: LengthPercentage::length(gap.value() as f32),
            height: LengthPercentage::length(gap.value() as f32),
        }
    }
}

impl From<&crate::features::layout_engine::domain::FlexStyle> for Style {
    fn from(style: &crate::features::layout_engine::domain::FlexStyle) -> Self {
        Style {
            flex_direction: style.direction().into(),
            justify_content: Some(style.justify().into()),
            align_items: Some(style.align_items().into()),
            position: style.position().into(),
            padding: style.padding().into(),
            margin: style.margin().into(),
            gap: style.gap().map(|g| g.into()).unwrap_or(TaffySize::zero()),
            ..Default::default()
        }
    }
}

fn node_to_style(node: &LayoutNode, measurer: &mut dyn TextMeasurer) -> Style {
    match node {
        LayoutNode::Flex { style, .. } => style.into(),
        LayoutNode::Text { text, font, size, .. } => {
            let measured = measurer.measure(text.as_str(), font.as_ref(), *size);
            Style {
                size: TaffySize {
                    width: Dimension::length(measured.width() as f32),
                    height: Dimension::length(measured.height() as f32),
                },
                ..Default::default()
            }
        }
        LayoutNode::Rect { size, .. } | LayoutNode::Image { size, .. } => Style {
            size: TaffySize {
                width: Dimension::length(size.width() as f32),
                height: Dimension::length(size.height() as f32),
            },
            ..Default::default()
        },
    }
}

struct TaffyTreeBuilder<'a> {
    taffy: &'a mut TaffyTree,
}

impl<'a> TaffyTreeBuilder<'a> {
    fn new(taffy: &'a mut TaffyTree) -> Self {
        Self { taffy }
    }

    fn add_leaf(&mut self, style: Style) -> Result<NodeId, LayoutError> {
        self.taffy.new_leaf(style).map_err(|e| LayoutError::EngineError(e.to_string()))
    }

    fn add_node(&mut self, style: Style, children: &[NodeId]) -> Result<NodeId, LayoutError> {
        self.taffy.new_with_children(style, children).map_err(|e| LayoutError::EngineError(e.to_string()))
    }

    fn set_style(&mut self, node_id: NodeId, style: Style) -> Result<(), LayoutError> {
        self.taffy.set_style(node_id, style).map_err(|e| LayoutError::EngineError(e.to_string()))
    }

    fn set_children(&mut self, node_id: NodeId, children: &[NodeId]) -> Result<(), LayoutError> {
        self.taffy.set_children(node_id, children).map_err(|e| LayoutError::EngineError(e.to_string()))
    }

    fn remove_recursive(&mut self, node_id: NodeId) {
        if let Ok(children) = self.taffy.children(node_id) {
            for child in children {
                self.remove_recursive(child);
            }
        }
        let _ = self.taffy.remove(node_id);
    }
}

impl LayoutEnginePort for TaffyLayoutAdapter {
    fn calculate_layout(
        &mut self,
        node: LayoutNode,
        measurer: &mut dyn TextMeasurer,
        start_pos: Position,
    ) -> Result<RenderNode, LayoutError> {
        let mut builder = TaffyTreeBuilder::new(&mut self.taffy);
        let new_state = if let Some(state) = &self.state {
            let patch = diff(state, &node, measurer);
            apply_patch(&mut builder, patch, measurer)?
        } else {
            build_layout_state(&mut builder, &node, measurer)?
        };

        let root_node_id = new_state.root_node;

        // Compute layout
        self.taffy
            .compute_layout(root_node_id, taffy::geometry::Size::MAX_CONTENT)
            .map_err(|e| LayoutError::EngineError(e.to_string()))?;

        // Build RenderNode tree
        let render_tree = build_render_tree(&self.taffy, root_node_id, &node, start_pos)?;

        self.state = Some(new_state);

        Ok(render_tree)
    }
}

fn build_layout_state(
    builder: &mut TaffyTreeBuilder,
    node: &LayoutNode,
    measurer: &mut dyn TextMeasurer,
) -> Result<LayoutState, LayoutError> {
    let style = node_to_style(node, measurer);

    match node {
        LayoutNode::Flex { children, .. } => {
            let mut state_children = Vec::new();
            let mut child_ids = Vec::new();
            for child in children {
                let state_child = build_layout_state(builder, child, measurer)?;
                child_ids.push(state_child.root_node);
                state_children.push(state_child);
            }

            let node_id = builder.add_node(style, &child_ids)?;

            Ok(LayoutState {
                root_node: node_id,
                layout: node.clone(),
                children: state_children,
            })
        }
        _ => {
            let node_id = builder.add_leaf(style)?;
            Ok(LayoutState {
                root_node: node_id,
                layout: node.clone(),
                children: Vec::new(),
            })
        }
    }
}

fn diff<'a>(
    old_state: &'a LayoutState,
    new_layout: &'a LayoutNode,
    measurer: &mut dyn TextMeasurer,
) -> Patch<'a> {
    if std::mem::discriminant(&old_state.layout) != std::mem::discriminant(new_layout) {
        return Patch::Replace {
            old_state,
            new_layout,
        };
    }

    match (&old_state.layout, new_layout) {
        (
            LayoutNode::Flex {
                style: old_style,
                ..
            },
            LayoutNode::Flex {
                style: new_style,
                children: new_children,
                ..
            },
        ) => {
            let style = if old_style != new_style {
                Some(node_to_style(new_layout, measurer))
            } else {
                None
            };

            let mut child_patches = Vec::new();
            for (i, new_child) in new_children.iter().enumerate() {
                if i < old_state.children.len() {
                    child_patches.push(diff(&old_state.children[i], new_child, measurer));
                } else {
                    child_patches.push(Patch::Create {
                        new_layout: new_child,
                    });
                }
            }

            Patch::Update {
                old_state,
                new_layout,
                style: Box::new(style),
                children: Some(child_patches),
            }
        }
        (
            LayoutNode::Text {
                text: old_text,
                font: old_font,
                size: old_size,
                ..
            },
            LayoutNode::Text {
                text: new_text,
                font: new_font,
                size: new_size,
                ..
            },
        ) => {
            let style = if old_text != new_text || old_font != new_font || old_size != new_size {
                Some(node_to_style(new_layout, measurer))
            } else {
                None
            };

            if style.is_some() {
                Patch::Update {
                    old_state,
                    new_layout,
                    style: Box::new(style),
                    children: None,
                }
            } else {
                Patch::Keep(old_state)
            }
        }
        (LayoutNode::Rect { size: old_size, .. }, LayoutNode::Rect { size: new_size, .. })
        | (LayoutNode::Image { size: old_size, .. }, LayoutNode::Image { size: new_size, .. }) => {
            let style = if old_size != new_size {
                Some(node_to_style(new_layout, measurer))
            } else {
                None
            };

            if style.is_some() {
                Patch::Update {
                    old_state,
                    new_layout,
                    style: Box::new(style),
                    children: None,
                }
            } else {
                Patch::Keep(old_state)
            }
        }
        _ => unreachable!(),
    }
}

fn apply_patch(
    builder: &mut TaffyTreeBuilder,
    patch: Patch,
    measurer: &mut dyn TextMeasurer,
) -> Result<LayoutState, LayoutError> {
    match patch {
        Patch::Keep(state) => Ok(state.clone()),
        Patch::Update {
            old_state,
            new_layout,
            style,
            children,
        } => {
            let node_id = old_state.root_node;

            if let Some(s) = *style {
                builder.set_style(node_id, s)?;
            }

            let mut new_state_children = Vec::new();
            if let Some(child_patches) = children {
                let mut new_child_ids = Vec::new();
                for cp in child_patches {
                    let child_state = apply_patch(builder, cp, measurer)?;
                    new_child_ids.push(child_state.root_node);
                    new_state_children.push(child_state);
                }

                if old_state.children.len() > new_state_children.len() {
                    for i in new_state_children.len()..old_state.children.len() {
                        builder.remove_recursive(old_state.children[i].root_node);
                    }
                }

                builder.set_children(node_id, &new_child_ids)?;
            }

            Ok(LayoutState {
                root_node: node_id,
                layout: new_layout.clone(),
                children: if new_state_children.is_empty() && !old_state.children.is_empty() {
                    old_state.children.clone()
                } else {
                    new_state_children
                },
            })
        }
        Patch::Replace {
            old_state,
            new_layout,
        } => {
            let new_state = build_layout_state(builder, new_layout, measurer)?;
            builder.remove_recursive(old_state.root_node);
            Ok(new_state)
        }
        Patch::Create { new_layout } => build_layout_state(builder, new_layout, measurer),
    }
}

fn build_render_tree(
    taffy: &TaffyTree,
    node_id: NodeId,
    node: &LayoutNode,
    offset: Position,
) -> Result<RenderNode, LayoutError> {
    let layout = taffy
        .layout(node_id)
        .map_err(|e| LayoutError::EngineError(e.to_string()))?;

    // Convert Taffy's relative coordinates into our Domain's absolute coordinates.
    let abs_x = offset.x() + layout.location.x as i32;
    let abs_y = offset.y() + layout.location.y as i32;
    let rect = Rect::new(
        Position::new(abs_x, abs_y),
        Size::new(layout.size.width as u32, layout.size.height as u32),
    );

    match node {
        LayoutNode::Flex {
            children,
            background,
            radius,
            on_click,
            on_hover,
            tooltip,
            ..
        } => {
            let child_ids = taffy
                .children(node_id)
                .map_err(|e| LayoutError::EngineError(e.to_string()))?;
            let mut render_children = Vec::new();

            for (child, &child_id) in children.iter().zip(child_ids.iter()) {
                render_children.push(build_render_tree(
                    taffy,
                    child_id,
                    child,
                    Position::new(abs_x, abs_y),
                )?);
            }

            Ok(RenderNode::Flex {
                rect,
                children: render_children,
                background: background.clone(),
                radius: *radius,
                on_click: on_click.clone(),
                on_hover: on_hover.clone(),
                tooltip: tooltip.clone(),
            })
        }
        LayoutNode::Text {
            text,
            color,
            font,
            size,
            on_click,
            on_hover,
            tooltip,
        } => Ok(RenderNode::Text {
            rect,
            text: text.clone(),
            color: color.clone(),
            font: font.clone(),
            size: *size,
            on_click: on_click.clone(),
            on_hover: on_hover.clone(),
            tooltip: tooltip.clone(),
        }),
        LayoutNode::Rect {
            color,
            radius,
            on_click,
            on_hover,
            tooltip,
            ..
        } => Ok(RenderNode::Rect {
            rect,
            color: color.clone(),
            radius: *radius,
            on_click: on_click.clone(),
            on_hover: on_hover.clone(),
            tooltip: tooltip.clone(),
        }),
        LayoutNode::Image {
            data, pixel_size, tooltip, ..
        } => Ok(RenderNode::Image {
            rect,
            data: data.clone(),
            pixel_size: *pixel_size,
            tooltip: tooltip.clone(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::layout_engine::domain::{
        FlexStyle, LayoutNode, RenderNode, TextContent, TextMeasurer,
    };
    use crate::shared::primitives::geometry::{Position, Size};
    use crate::shared::primitives::color::DrawingColor;
    use crate::shared::config::domain::FontFamily;
    use crate::shared::config::domain::FontSize;

    struct MockMeasurer;
    impl TextMeasurer for MockMeasurer {
        fn measure(&mut self, text: &str, _font: Option<&FontFamily>, _size: Option<FontSize>) -> Size {
            Size::new(text.len() as u32 * 10, 20)
        }
    }

    #[test]
    fn test_node_to_style_flex() {
        let mut measurer = MockMeasurer;
        let node = LayoutNode::Flex {
            children: vec![],
            style: FlexStyle::default(),
            background: None,
            radius: None,
            on_click: None,
            on_hover: None, tooltip: None,
        };
        let style = node_to_style(&node, &mut measurer);
        assert_eq!(style.flex_direction, taffy::style::FlexDirection::Row);
    }

    #[test]
    fn test_calculate_layout_simple_rect() {
        let mut adapter = TaffyLayoutAdapter::new();
        let mut measurer = MockMeasurer;
        
        let node = LayoutNode::Rect {
            size: Size::new(100, 50),
            color: DrawingColor::default(),
            radius: None,
            on_click: None,
            on_hover: None, tooltip: None,
        };
        
        let render_tree = adapter.calculate_layout(node, &mut measurer, Position::new(10, 10)).unwrap();
        
        if let RenderNode::Rect { rect, .. } = render_tree {
            assert_eq!(rect.x(), 10);
            assert_eq!(rect.y(), 10);
            assert_eq!(rect.width(), 100);
            assert_eq!(rect.height(), 50);
        } else {
            panic!("Expected RenderNode::Rect");
        }
    }

    #[test]
    fn test_calculate_layout_flex_with_children() {
        let mut adapter = TaffyLayoutAdapter::new();
        let mut measurer = MockMeasurer;
        
        let child1 = LayoutNode::Rect {
            size: Size::new(50, 50),
            color: DrawingColor::default(),
            radius: None,
            on_click: None,
            on_hover: None, tooltip: None,
        };
        
        let child2 = LayoutNode::Text {
            text: TextContent::new("hello".to_string()),
            color: DrawingColor::default(),
            font: None,
            size: None,
            on_click: None,
            on_hover: None, tooltip: None,
        };
        
        let parent = LayoutNode::Flex {
            children: vec![child1, child2],
            style: FlexStyle::default(),
            background: None,
            radius: None,
            on_click: None,
            on_hover: None, tooltip: None,
        };
        
        let render_tree = adapter.calculate_layout(parent, &mut measurer, Position::new(0, 0)).unwrap();
        
        if let RenderNode::Flex { rect, children, .. } = render_tree {
            assert_eq!(rect.width(), 100);
            assert_eq!(rect.height(), 50);
            assert_eq!(children.len(), 2);
            
            assert_eq!(children[0].rect().x(), 0);
            assert_eq!(children[0].rect().y(), 0);
            
            assert_eq!(children[1].rect().x(), 50);
            assert_eq!(children[1].rect().y(), 0);
        } else {
            panic!("Expected RenderNode::Flex");
        }
    }

    #[test]
    fn test_calculate_layout_diffing() {
        let mut adapter = TaffyLayoutAdapter::new();
        let mut measurer = MockMeasurer;
        
        let node1 = LayoutNode::Rect {
            size: Size::new(100, 50),
            color: DrawingColor::default(),
            radius: None,
            on_click: None,
            on_hover: None, tooltip: None,
        };
        adapter.calculate_layout(node1.clone(), &mut measurer, Position::new(0, 0)).unwrap();
        
        let node2 = LayoutNode::Rect {
            size: Size::new(200, 50),
            color: DrawingColor::default(),
            radius: None,
            on_click: None,
            on_hover: None, tooltip: None,
        };
        let render2 = adapter.calculate_layout(node2.clone(), &mut measurer, Position::new(0, 0)).unwrap();
        assert_eq!(render2.rect().width(), 200);
        
        let node3 = LayoutNode::Text {
            text: TextContent::new("hello".to_string()),
            color: DrawingColor::default(),
            font: None,
            size: None,
            on_click: None,
            on_hover: None, tooltip: None,
        };
        let render3 = adapter.calculate_layout(node3.clone(), &mut measurer, Position::new(0, 0)).unwrap();
        assert_eq!(render3.rect().width(), 50);
        
        // Test updating text node style
        let node4 = LayoutNode::Text {
            text: TextContent::new("hello world".to_string()),
            color: DrawingColor::default(),
            font: None,
            size: None,
            on_click: None,
            on_hover: None, tooltip: None,
        };
        let render4 = adapter.calculate_layout(node4.clone(), &mut measurer, Position::new(0, 0)).unwrap();
        assert_eq!(render4.rect().width(), 110);
        
        // Test Image -> Image with diff sizes
        let node5 = LayoutNode::Image {
            data: vec![],
            pixel_size: Size::new(10, 10),
            size: Size::new(20, 20),
            tooltip: None,
        };
        let render5 = adapter.calculate_layout(node5.clone(), &mut measurer, Position::new(0, 0)).unwrap();
        assert_eq!(render5.rect().width(), 20);

        let node6 = LayoutNode::Image {
            data: vec![],
            pixel_size: Size::new(10, 10),
            size: Size::new(30, 30),
            tooltip: None,
        };
        let render6 = adapter.calculate_layout(node6.clone(), &mut measurer, Position::new(0, 0)).unwrap();
        assert_eq!(render6.rect().width(), 30);

        // Test Flex shrinking children
        let parent1 = LayoutNode::Flex {
            children: vec![node1.clone(), node2.clone()],
            style: FlexStyle::default(),
            background: None,
            radius: None,
            on_click: None,
            on_hover: None, tooltip: None,
        };
        adapter.calculate_layout(parent1.clone(), &mut measurer, Position::new(0, 0)).unwrap();

        let parent2 = LayoutNode::Flex {
            children: vec![node1.clone()],
            style: FlexStyle::default(),
            background: None,
            radius: None,
            on_click: None,
            on_hover: None, tooltip: None,
        };
        adapter.calculate_layout(parent2.clone(), &mut measurer, Position::new(0, 0)).unwrap();

        // Test Flex growing children
        let parent3 = LayoutNode::Flex {
            children: vec![node1.clone(), node2.clone(), node3.clone()],
            style: FlexStyle::default(),
            background: None,
            radius: None,
            on_click: None,
            on_hover: None, tooltip: None,
        };
        adapter.calculate_layout(parent3.clone(), &mut measurer, Position::new(0, 0)).unwrap();
        
        // Test Flex keep children but update style
        let flex_style_modified: FlexStyle = serde_json::from_value(serde_json::json!({
            "gap": 15.0
        })).unwrap();
        let parent4 = LayoutNode::Flex {
            children: vec![node1.clone(), node2.clone(), node3.clone()],
            style: flex_style_modified,
            background: Some(crate::shared::primitives::color::DrawingColor::parse("#000000").unwrap()),
            radius: None,
            on_click: None,
            on_hover: None, tooltip: None,
        };
        adapter.calculate_layout(parent4.clone(), &mut measurer, Position::new(0, 0)).unwrap();

        // Keep Text
        adapter.calculate_layout(node3.clone(), &mut measurer, Position::new(0, 0)).unwrap();
        adapter.calculate_layout(node3.clone(), &mut measurer, Position::new(0, 0)).unwrap();
        
        // Keep Image
        adapter.calculate_layout(node6.clone(), &mut measurer, Position::new(0, 0)).unwrap();
        adapter.calculate_layout(node6.clone(), &mut measurer, Position::new(0, 0)).unwrap();
    }
}
