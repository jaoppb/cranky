use crate::ports::layout::LayoutEnginePort;
use crate::domain::layout::{LayoutNode, RenderNode, TextMeasurer, FlexDirection, JustifyContent, AlignItems};
use crate::domain::shared::geometry::{Position, Rect, Size};
use taffy::{
    style::{Style, FlexDirection as TaffyFlexDirection, JustifyContent as TaffyJustifyContent, AlignItems as TaffyAlignItems},
    geometry::Size as TaffySize,
    tree::NodeId,
    TaffyTree,
    style::LengthPercentage,
    style::Dimension,
};
use taffy::prelude::TaffyMaxContent;
use crate::domain::layout::LayoutError;

pub struct TaffyLayoutAdapter;

impl TaffyLayoutAdapter {
    pub fn new() -> Self {
        Self
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

impl From<crate::domain::layout::PositionType> for taffy::style::Position {
    fn from(pt: crate::domain::layout::PositionType) -> Self {
        match pt {
            crate::domain::layout::PositionType::Relative => taffy::style::Position::Relative,
            crate::domain::layout::PositionType::Absolute => taffy::style::Position::Absolute,
        }
    }
}

impl From<&crate::domain::layout::BoxMargin> for taffy::geometry::Rect<LengthPercentage> {
    fn from(padding: &crate::domain::layout::BoxMargin) -> Self {
        taffy::geometry::Rect {
            left: LengthPercentage::length(padding.left() as f32),
            right: LengthPercentage::length(padding.right() as f32),
            top: LengthPercentage::length(padding.top() as f32),
            bottom: LengthPercentage::length(padding.bottom() as f32),
        }
    }
}

impl From<&crate::domain::layout::BoxMargin> for taffy::geometry::Rect<taffy::style::LengthPercentageAuto> {
    fn from(margin: &crate::domain::layout::BoxMargin) -> Self {
        taffy::geometry::Rect {
            left: LengthPercentage::length(margin.left() as f32).into(),
            right: LengthPercentage::length(margin.right() as f32).into(),
            top: LengthPercentage::length(margin.top() as f32).into(),
            bottom: LengthPercentage::length(margin.bottom() as f32).into(),
        }
    }
}

impl From<&crate::domain::layout::Gap> for TaffySize<LengthPercentage> {
    fn from(gap: &crate::domain::layout::Gap) -> Self {
        TaffySize {
            width: LengthPercentage::length(gap.value() as f32),
            height: LengthPercentage::length(gap.value() as f32),
        }
    }
}

impl From<&crate::domain::layout::FlexStyle> for Style {
    fn from(style: &crate::domain::layout::FlexStyle) -> Self {
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

impl LayoutEnginePort for TaffyLayoutAdapter {
    fn calculate_layout(
        &self,
        node: LayoutNode,
        measurer: &mut dyn TextMeasurer,
        start_pos: Position,
    ) -> Result<RenderNode, LayoutError> {
        let mut taffy = TaffyTree::new();
        
        let root_node_id = build_taffy_tree(&mut taffy, &node, measurer)?;
        
        // Compute layout
        taffy.compute_layout(root_node_id, taffy::geometry::Size::MAX_CONTENT)
            .map_err(|e| LayoutError::EngineError(e.to_string()))?;
        
        // Build RenderNode tree
        build_render_tree(&taffy, root_node_id, &node, start_pos)
    }
}

fn build_taffy_tree(
    taffy: &mut TaffyTree,
    node: &LayoutNode,
    measurer: &mut dyn TextMeasurer,
) -> Result<NodeId, LayoutError> {
    match node {
        LayoutNode::Flex { style, children, .. } => {
            let taffy_style: Style = style.into();
            
            let mut child_nodes = Vec::new();
            for child in children {
                child_nodes.push(build_taffy_tree(taffy, child, measurer)?);
            }
            
            taffy.new_with_children(taffy_style, &child_nodes)
                .map_err(|e| LayoutError::EngineError(e.to_string()))
        }
        LayoutNode::Text { text, font, size, .. } => {
            let measured = measurer.measure(text.as_str(), font.as_ref(), *size);
            let style = Style {
                size: TaffySize {
                    width: Dimension::length(measured.width() as f32),
                    height: Dimension::length(measured.height() as f32),
                },
                ..Default::default()
            };
            taffy.new_leaf(style).map_err(|e| LayoutError::EngineError(e.to_string()))
        }
        LayoutNode::Rect { size, .. } | LayoutNode::Image { size, .. } => {
            let style = Style {
                size: TaffySize {
                    width: Dimension::length(size.width() as f32),
                    height: Dimension::length(size.height() as f32),
                },
                ..Default::default()
            };
            taffy.new_leaf(style).map_err(|e| LayoutError::EngineError(e.to_string()))
        }
    }
}

fn build_render_tree(
    taffy: &TaffyTree,
    node_id: NodeId,
    node: &LayoutNode,
    offset: Position,
) -> Result<RenderNode, LayoutError> {
    let layout = taffy.layout(node_id).map_err(|e| LayoutError::EngineError(e.to_string()))?;
    
    // Convert Taffy's relative coordinates into our Domain's absolute coordinates.
    let abs_x = offset.x() + layout.location.x as i32;
    let abs_y = offset.y() + layout.location.y as i32;
    let rect = Rect::new(
        Position::new(abs_x, abs_y),
        Size::new(layout.size.width as u32, layout.size.height as u32),
    );

    match node {
        LayoutNode::Flex { children, background, radius, on_click, on_hover, .. } => {
            let child_ids = taffy.children(node_id).map_err(|e| LayoutError::EngineError(e.to_string()))?;
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
            })
        }
        LayoutNode::Text { text, color, font, size, on_click, on_hover } => {
            Ok(RenderNode::Text {
                rect,
                text: text.clone(),
                color: color.clone(),
                font: font.clone(),
                size: *size,
                on_click: on_click.clone(),
                on_hover: on_hover.clone(),
            })
        }
        LayoutNode::Rect { color, radius, on_click, on_hover, .. } => {
            Ok(RenderNode::Rect {
                rect,
                color: color.clone(),
                radius: *radius,
                on_click: on_click.clone(),
                on_hover: on_hover.clone(),
            })
        }
        LayoutNode::Image { data, pixel_size, .. } => {
            Ok(RenderNode::Image {
                rect,
                data: data.clone(),
                pixel_size: *pixel_size,
            })
        }
    }
}
