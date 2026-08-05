use crate::domain::shared::color::DrawingColor;
use crate::domain::shared::geometry::Size;
use crate::domain::config::{FontFamily, FontSize, BorderRadius};
use crate::domain::commands::AppCommand;
use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum LayoutError {
    #[error("Failed to compute layout: {0}")]
    EngineError(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum FlexDirection {
    #[default]
    Row,
    Column,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum JustifyContent {
    #[default]
    Start,
    End,
    Center,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AlignItems {
    #[default]
    Start,
    End,
    Center,
    Stretch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PositionType {
    #[default]
    Relative,
    Absolute,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct Gap {
    value: f64,
}

impl Gap {
    pub fn new(value: f64) -> Self { Self { value } }
    pub fn value(&self) -> f64 {
        self.value
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Default)]
pub struct BoxMargin {
    #[serde(default)]
    top: f64,
    #[serde(default)]
    bottom: f64,
    #[serde(default)]
    left: f64,
    #[serde(default)]
    right: f64,
}

impl BoxMargin {
    pub fn new(top: f64, bottom: f64, left: f64, right: f64) -> Self {
        Self { top, bottom, left, right }
    }
}

impl BoxMargin {
    pub fn top(&self) -> f64 {
        self.top
    }
    pub fn bottom(&self) -> f64 {
        self.bottom
    }
    pub fn left(&self) -> f64 {
        self.left
    }
    pub fn right(&self) -> f64 {
        self.right
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Default)]
pub struct FlexStyle {
    #[serde(default)]
    direction: FlexDirection,
    #[serde(default)]
    justify: JustifyContent,
    #[serde(default)]
    align_items: AlignItems,
    #[serde(default)]
    padding: BoxMargin,
    #[serde(default)]
    margin: BoxMargin,
    #[serde(default)]
    gap: Option<Gap>,
    #[serde(default)]
    position: PositionType,
}

impl FlexStyle {
    pub fn with_padding(mut self, padding: BoxMargin) -> Self {
        self.padding = padding;
        self
    }
    pub fn direction(&self) -> FlexDirection { self.direction }
    pub fn justify(&self) -> JustifyContent { self.justify }
    pub fn align_items(&self) -> AlignItems { self.align_items }
    pub fn padding(&self) -> &BoxMargin { &self.padding }
    pub fn margin(&self) -> &BoxMargin { &self.margin }
    pub fn gap(&self) -> Option<&Gap> { self.gap.as_ref() }
    pub fn position(&self) -> PositionType { self.position }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct TextContent {
    text: String,
}

impl TextContent {
    pub fn new(text: String) -> Self {
        Self { text }
    }
    pub fn as_str(&self) -> &str {
        &self.text
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "type")]
pub enum LayoutNode {
    #[serde(rename = "flex")]
    Flex {
        #[serde(default)]
        children: Vec<LayoutNode>,
        #[serde(default)]
        style: FlexStyle,
        #[serde(default)]
        background: Option<DrawingColor>,
        #[serde(default)]
        radius: Option<BorderRadius>,
        on_click: Option<AppCommand>,
        on_hover: Option<AppCommand>,
        #[serde(default)]
        tooltip: Option<Box<LayoutNode>>,
    },
    #[serde(rename = "text")]
    Text {
        text: TextContent,
        color: DrawingColor,
        font: Option<FontFamily>,
        size: Option<FontSize>,
        on_click: Option<AppCommand>,
        on_hover: Option<AppCommand>,
        #[serde(default)]
        tooltip: Option<Box<LayoutNode>>,
    },
    #[serde(rename = "rect")]
    Rect {
        size: Size,
        color: DrawingColor,
        radius: Option<BorderRadius>,
        on_click: Option<AppCommand>,
        on_hover: Option<AppCommand>,
        #[serde(default)]
        tooltip: Option<Box<LayoutNode>>,
    },
    #[serde(rename = "image")]
    Image {
        size: Size,
        #[serde(with = "serde_bytes")]
        data: Vec<u8>,
        pixel_size: Size,
        #[serde(default)]
        tooltip: Option<Box<LayoutNode>>,
    }
}

pub trait TextMeasurer: Send + Sync {
    fn measure(&mut self, text: &str, font: Option<&FontFamily>, size: Option<FontSize>) -> Size;
}

use crate::domain::shared::geometry::Rect;

#[derive(Debug, Clone, PartialEq)]
pub enum RenderNode {
    Flex {
        rect: Rect,
        children: Vec<RenderNode>,
        background: Option<DrawingColor>,
        radius: Option<BorderRadius>,
        on_click: Option<AppCommand>,
        on_hover: Option<AppCommand>,
        tooltip: Option<Box<LayoutNode>>,
    },
    Text {
        rect: Rect,
        text: TextContent,
        color: DrawingColor,
        font: Option<FontFamily>,
        size: Option<FontSize>,
        on_click: Option<AppCommand>,
        on_hover: Option<AppCommand>,
        tooltip: Option<Box<LayoutNode>>,
    },
    Rect {
        rect: Rect,
        color: DrawingColor,
        radius: Option<BorderRadius>,
        on_click: Option<AppCommand>,
        on_hover: Option<AppCommand>,
        tooltip: Option<Box<LayoutNode>>,
    },
    Image {
        rect: Rect,
        data: Vec<u8>,
        pixel_size: Size,
        tooltip: Option<Box<LayoutNode>>,
    }
}

impl RenderNode {
    pub fn rect(&self) -> Rect {
        match self {
            Self::Flex { rect, .. } => *rect,
            Self::Text { rect, .. } => *rect,
            Self::Rect { rect, .. } => *rect,
            Self::Image { rect, .. } => *rect,
        }
    }

    pub fn on_click(&self) -> Option<&crate::domain::commands::AppCommand> {
        match self {
            Self::Text { on_click, .. } => on_click.as_ref(),
            Self::Flex { on_click, .. } => on_click.as_ref(),
            Self::Rect { on_click, .. } => on_click.as_ref(),
            _ => None,
        }
    }

    pub fn on_hover(&self) -> Option<&crate::domain::commands::AppCommand> {
        match self {
            Self::Text { on_hover, .. } => on_hover.as_ref(),
            Self::Flex { on_hover, .. } => on_hover.as_ref(),
            Self::Rect { on_hover, .. } => on_hover.as_ref(),
            _ => None,
        }
    }

    pub fn hit_test(&self, pos: crate::domain::shared::geometry::Position) -> Vec<&RenderNode> {
        let mut path = Vec::new();
        self.hit_test_internal(pos, &mut path);
        path
    }

    fn hit_test_internal<'a>(&'a self, pos: crate::domain::shared::geometry::Position, path: &mut Vec<&'a RenderNode>) {
        let r = self.rect();
        if pos.x() >= r.x() && pos.x() < r.x() + r.width() as i32 && pos.y() >= r.y() && pos.y() < r.y() + r.height() as i32 {
            path.push(self);
            if let Self::Flex { children, .. } = self {
                for child in children {
                    let prev_len = path.len();
                    child.hit_test_internal(pos, path);
                    if path.len() > prev_len {
                        break;
                    }
                }
            }
        }
    }

    pub fn tooltip(&self) -> Option<&LayoutNode> {
        match self {
            RenderNode::Flex { tooltip, .. } => tooltip.as_deref(),
            RenderNode::Text { tooltip, .. } => tooltip.as_deref(),
            RenderNode::Rect { tooltip, .. } => tooltip.as_deref(),
            RenderNode::Image { tooltip, .. } => tooltip.as_deref(),
        }
    }

    pub fn render_to_canvas(&self, canvas: &mut dyn crate::ports::canvas::Canvas) {
        use crate::domain::shared::geometry::LogicalPx;
        match self {
            Self::Flex { rect, children, background, radius, .. } => {
                if let Some(bg) = background {
                    canvas.draw_rect(
                        LogicalPx::new(rect.x() as f32),
                        LogicalPx::new(rect.y() as f32),
                        LogicalPx::new(rect.width() as f32),
                        LogicalPx::new(rect.height() as f32),
                        bg.clone(),
                        LogicalPx::new(radius.map(|r| r.value()).unwrap_or(0.0)),
                    );
                }
                for child in children {
                    child.render_to_canvas(canvas);
                }
            }
            Self::Rect { rect, color, radius, .. } => {
                canvas.draw_rect(
                    LogicalPx::new(rect.x() as f32),
                    LogicalPx::new(rect.y() as f32),
                    LogicalPx::new(rect.width() as f32),
                    LogicalPx::new(rect.height() as f32),
                    color.clone(),
                    LogicalPx::new(radius.map(|r| r.value()).unwrap_or(0.0)),
                );
            }
            Self::Text { rect, text, color, font, size, .. } => {
                canvas.draw_text(
                    text.as_str(),
                    font.as_ref(),
                    *size,
                    color.clone(),
                    crate::domain::shared::geometry::Position::new(rect.x(), rect.y()),
                );
            }
            Self::Image { rect, data, pixel_size, tooltip: _ } => {
                let logical_size = crate::domain::shared::geometry::Size::new(rect.width(), rect.height());
                canvas.draw_image(
                    data,
                    *pixel_size,
                    logical_size,
                    crate::domain::shared::geometry::Position::new(rect.x(), rect.y())
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::shared::geometry::Position;
    use crate::domain::shared::color::Color;

    #[test]
    fn test_gap_and_margins() {
        let gap = Gap { value: 10.0 };
        assert_eq!(gap.value(), 10.0);

        let margin = BoxMargin { top: 1.0, bottom: 2.0, left: 3.0, right: 4.0 };
        assert_eq!(margin.top(), 1.0);
        assert_eq!(margin.bottom(), 2.0);
        assert_eq!(margin.left(), 3.0);
        assert_eq!(margin.right(), 4.0);
    }

    #[test]
    fn test_flex_style() {
        let style = FlexStyle {
            direction: FlexDirection::Column,
            justify: JustifyContent::Center,
            align_items: AlignItems::Stretch,
            position: PositionType::Absolute,
            padding: BoxMargin { top: 1.0, bottom: 1.0, left: 1.0, right: 1.0 },
            margin: BoxMargin::default(),
            gap: Some(Gap { value: 5.0 }),
        };

        assert_eq!(style.direction(), FlexDirection::Column);
        assert_eq!(style.justify(), JustifyContent::Center);
        assert_eq!(style.align_items(), AlignItems::Stretch);
        assert_eq!(style.position(), PositionType::Absolute);
        assert_eq!(style.padding().top(), 1.0);
        assert_eq!(style.margin().top(), 0.0);
        assert_eq!(style.gap().unwrap().value(), 5.0);
    }

    #[test]
    fn test_text_content() {
        let text = TextContent::new("hello".to_string());
        assert_eq!(text.as_str(), "hello");
    }

    #[test]
    fn test_render_node_accessors() {
        let rect = Rect::new(Position::new(0, 0), Size::new(10, 10));
        let node = RenderNode::Rect {
            rect,
            color: DrawingColor::Solid(Color::new(0, 0, 0, 255)),
            radius: None,
            on_click: None,
            on_hover: None,
            tooltip: None,
        };
        assert_eq!(node.rect(), rect);
        assert_eq!(node.on_click(), None);
        assert_eq!(node.on_hover(), None);
    }

    #[test]
    fn test_hit_test() {
        let rect1 = Rect::new(Position::new(0, 0), Size::new(100, 100));
        let rect2 = Rect::new(Position::new(10, 10), Size::new(50, 50));
        
        let child_node = RenderNode::Rect {
            rect: rect2,
            color: DrawingColor::Solid(Color::new(255, 0, 0, 255)),
            radius: None,
            on_click: None,
            on_hover: None,
            tooltip: None,
        };

        let parent_node = RenderNode::Flex {
            rect: rect1,
            children: vec![child_node.clone()],
            background: None,
            radius: None,
            on_click: None,
            on_hover: None,
            tooltip: None,
        };

        // Hit child
        let hit = parent_node.hit_test(Position::new(20, 20));
        assert_eq!(*hit.last().unwrap(), &child_node);

        // Hit parent only
        let hit2 = parent_node.hit_test(Position::new(80, 80));
        assert_eq!(*hit2.last().unwrap(), &parent_node);

        // Miss
        assert!(parent_node.hit_test(Position::new(200, 200)).is_empty());
    }

    #[test]
    fn test_render_to_canvas() {
        use crate::ports::canvas::MockCanvas;
        
        let mut canvas = MockCanvas::new();
        canvas.expect_draw_rect().times(1).return_const(());
        
        let rect = RenderNode::Rect {
            rect: Rect::new(Position::new(0, 0), Size::new(10, 10)),
            color: DrawingColor::Solid(Color::new(255, 255, 255, 255)),
            radius: None,
            on_click: None,
            on_hover: None,
            tooltip: None,
        };
        
        rect.render_to_canvas(&mut canvas);
        
        let mut canvas = MockCanvas::new();
        canvas.expect_draw_rect().times(1).return_const(());
        let flex = RenderNode::Flex {
            rect: Rect::new(Position::new(0, 0), Size::new(10, 10)),
            children: vec![],
            background: Some(DrawingColor::Solid(Color::new(0, 0, 0, 255))),
            radius: None,
            on_click: None,
            on_hover: None,
            tooltip: None,
        };
        flex.render_to_canvas(&mut canvas);
        
        let mut canvas = MockCanvas::new();
        canvas.expect_draw_text().times(1).return_const(());
        let text = RenderNode::Text {
            rect: Rect::new(Position::new(0, 0), Size::new(10, 10)),
            text: TextContent::new("test".into()),
            color: DrawingColor::Solid(Color::new(0, 0, 0, 255)),
            font: None,
            size: None,
            on_click: None,
            on_hover: None,
            tooltip: None,
        };
        text.render_to_canvas(&mut canvas);
        
        let mut canvas = MockCanvas::new();
        canvas.expect_draw_image().times(1).return_const(());
        let image = RenderNode::Image {
            rect: Rect::new(Position::new(0, 0), Size::new(10, 10)),
            data: vec![0,0,0,0],
            pixel_size: Size::new(1, 1),
            tooltip: None,
        };
        image.render_to_canvas(&mut canvas);
    }
}
