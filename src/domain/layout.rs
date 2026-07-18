use crate::domain::shared::color::DrawingColor;
use crate::domain::shared::geometry::Size;
use crate::domain::config::{FontFamily, FontSize, BorderRadius};
use crate::domain::commands::AppCommand;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AlignX {
    Left,
    #[default]
    Center,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AlignY {
    Top,
    #[default]
    Center,
    Bottom,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct Gap(pub f64);

impl Gap {
    pub fn value(&self) -> f64 {
        self.0
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

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct TextContent(pub String);

impl TextContent {
    pub fn new(text: String) -> Self {
        Self(text)
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "type")]
pub enum LayoutNode {
    #[serde(rename = "row")]
    Row {
        #[serde(default)]
        children: Vec<LayoutNode>,
        #[serde(default)]
        gap: Option<Gap>,
        #[serde(default)]
        align_items: AlignY,
        on_click: Option<AppCommand>,
        on_hover: Option<AppCommand>,
    },
    #[serde(rename = "text")]
    Text {
        text: TextContent,
        color: DrawingColor,
        font: Option<FontFamily>,
        size: Option<FontSize>,
        on_click: Option<AppCommand>,
        on_hover: Option<AppCommand>,
    },
    #[serde(rename = "rect")]
    Rect {
        size: Size,
        color: DrawingColor,
        radius: Option<BorderRadius>,
        on_click: Option<AppCommand>,
        on_hover: Option<AppCommand>,
    },
    #[serde(rename = "image")]
    Image {
        size: Size,
        data: Vec<u8>,
        pixel_size: Size,
    },
    #[serde(rename = "box")]
    Box {
        child: Box<LayoutNode>,
        #[serde(default)]
        background: Option<DrawingColor>,
        #[serde(default)]
        radius: Option<BorderRadius>,
        #[serde(default)]
        margin: Option<BoxMargin>,
        #[serde(default)]
        padding: Option<Gap>,
        on_click: Option<AppCommand>,
        on_hover: Option<AppCommand>,
    },
    #[serde(rename = "stack")]
    Stack {
        children: Vec<LayoutNode>,
        #[serde(default)]
        align_x: AlignX,
        #[serde(default)]
        align_y: AlignY,
        on_click: Option<AppCommand>,
        on_hover: Option<AppCommand>,
    }
}

pub trait TextMeasurer {
    fn measure(&mut self, text: &str, font: Option<&FontFamily>, size: Option<FontSize>) -> Size;
}

use crate::domain::shared::geometry::{Position, Rect};

#[derive(Debug, Clone, PartialEq)]
pub enum RenderNode {
    Row {
        rect: Rect,
        children: Vec<RenderNode>,
        align_items: AlignY,
        on_click: Option<AppCommand>,
        on_hover: Option<AppCommand>,
    },
    Text {
        rect: Rect,
        text: TextContent,
        color: DrawingColor,
        font: Option<FontFamily>,
        size: Option<FontSize>,
        on_click: Option<AppCommand>,
        on_hover: Option<AppCommand>,
    },
    Rect {
        rect: Rect,
        color: DrawingColor,
        radius: Option<BorderRadius>,
        on_click: Option<AppCommand>,
        on_hover: Option<AppCommand>,
    },
    Image {
        rect: Rect,
        data: Vec<u8>,
        pixel_size: Size,
    },
    Box {
        rect: Rect,
        child: Box<RenderNode>,
        background: Option<DrawingColor>,
        radius: Option<BorderRadius>,
        margin: Option<BoxMargin>,
        padding: Option<Gap>,
        on_click: Option<AppCommand>,
        on_hover: Option<AppCommand>,
    },
    Stack {
        rect: Rect,
        children: Vec<RenderNode>,
        align_x: AlignX,
        align_y: AlignY,
        on_click: Option<AppCommand>,
        on_hover: Option<AppCommand>,
    }
}

impl RenderNode {
    pub fn rect(&self) -> Rect {
        match self {
            Self::Row { rect, .. } => *rect,
            Self::Text { rect, .. } => *rect,
            Self::Rect { rect, .. } => *rect,
            Self::Image { rect, .. } => *rect,
            Self::Box { rect, .. } => *rect,
            Self::Stack { rect, .. } => *rect,
        }
    }

    pub fn translate(&mut self, dx: i32, dy: i32) {
        if dx == 0 && dy == 0 {
            return;
        }
        match self {
            Self::Row { rect, children, .. } => {
                rect.set_x(rect.x() + dx);
                rect.set_y(rect.y() + dy);
                for child in children {
                    child.translate(dx, dy);
                }
            }
            Self::Text { rect, .. } => {
                rect.set_x(rect.x() + dx);
                rect.set_y(rect.y() + dy);
            }
            Self::Rect { rect, .. } => {
                rect.set_x(rect.x() + dx);
                rect.set_y(rect.y() + dy);
            }
            Self::Image { rect, .. } => {
                rect.set_x(rect.x() + dx);
                rect.set_y(rect.y() + dy);
            }
            Self::Box { rect, child, .. } => {
                rect.set_x(rect.x() + dx);
                rect.set_y(rect.y() + dy);
                child.translate(dx, dy);
            }
            Self::Stack { rect, children, .. } => {
                rect.set_x(rect.x() + dx);
                rect.set_y(rect.y() + dy);
                for child in children {
                    child.translate(dx, dy);
                }
            }
        }
    }

    pub fn on_click(&self) -> Option<&crate::domain::commands::AppCommand> {
        match self {
            Self::Text { on_click, .. } => on_click.as_ref(),
            Self::Row { on_click, .. } => on_click.as_ref(),
            Self::Rect { on_click, .. } => on_click.as_ref(),
            Self::Box { on_click, .. } => on_click.as_ref(),
            Self::Stack { on_click, .. } => on_click.as_ref(),
            _ => None,
        }
    }

    pub fn on_hover(&self) -> Option<&crate::domain::commands::AppCommand> {
        match self {
            Self::Text { on_hover, .. } => on_hover.as_ref(),
            Self::Row { on_hover, .. } => on_hover.as_ref(),
            Self::Rect { on_hover, .. } => on_hover.as_ref(),
            Self::Box { on_hover, .. } => on_hover.as_ref(),
            Self::Stack { on_hover, .. } => on_hover.as_ref(),
            _ => None,
        }
    }

    pub fn hit_test(&self, pos: crate::domain::shared::geometry::Position) -> Option<&RenderNode> {
        let r = self.rect();
        if pos.x() >= r.x() && pos.x() < r.x() + r.width() as i32 && pos.y() >= r.y() && pos.y() < r.y() + r.height() as i32 {
            if let Self::Row { children, .. } = self {
                for child in children {
                    if let Some(hit) = child.hit_test(pos) {
                        return Some(hit);
                    }
                }
            } else if let Self::Stack { children, .. } = self {
                for child in children.iter().rev() {
                    if let Some(hit) = child.hit_test(pos) {
                        return Some(hit);
                    }
                }
            } else if let Self::Box { child, .. } = self
                && let Some(hit) = child.hit_test(pos) {
                    return Some(hit);
                }
            Some(self)
        } else {
            None
        }
    }

    pub fn render_to_canvas(&self, canvas: &mut dyn crate::ports::canvas::Canvas) {
        use crate::domain::shared::geometry::LogicalPx;
        match self {
            Self::Row { children, .. } => {
                for child in children {
                    child.render_to_canvas(canvas);
                }
            }
            Self::Stack { children, .. } => {
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
            Self::Image { rect, data, pixel_size } => {
                let logical_size = crate::domain::shared::geometry::Size::new(rect.width(), rect.height());
                canvas.draw_image(
                    data,
                    *pixel_size,
                    logical_size,
                    crate::domain::shared::geometry::Position::new(rect.x(), rect.y())
                );
            }
            Self::Box { rect, child, background, radius, margin, .. } => {
                if let Some(bg) = background {
                    let mt = margin.as_ref().map(|m| m.top()).unwrap_or(0.0);
                    let mb = margin.as_ref().map(|m| m.bottom()).unwrap_or(0.0);
                    let ml = margin.as_ref().map(|m| m.left()).unwrap_or(0.0);
                    let mr = margin.as_ref().map(|m| m.right()).unwrap_or(0.0);

                    canvas.draw_rect(
                        LogicalPx::new(rect.x() as f32 + ml as f32),
                        LogicalPx::new(rect.y() as f32 + mt as f32),
                        LogicalPx::new(rect.width() as f32 - (ml + mr) as f32),
                        LogicalPx::new(rect.height() as f32 - (mt + mb) as f32),
                        bg.clone(),
                        LogicalPx::new(radius.map(|r| r.value()).unwrap_or(0.0)),
                    );
                }
                child.render_to_canvas(canvas);
            }
        }
    }
}

pub struct LayoutEngine;

impl LayoutEngine {
    pub fn layout(node: LayoutNode, measurer: &mut dyn TextMeasurer, start_pos: Position) -> RenderNode {
        match node {
            LayoutNode::Text { text, color, font, size, on_click, on_hover } => {
                let measured_size = measurer.measure(text.as_str(), font.as_ref(), size);
                let rect = Rect::new(start_pos, measured_size);
                RenderNode::Text { rect, text, color, font, size, on_click, on_hover }
            }
            LayoutNode::Rect { size, color, radius, on_click, on_hover } => {
                let rect = Rect::new(start_pos, size);
                RenderNode::Rect { rect, color, radius, on_click, on_hover }
            }
            LayoutNode::Image { size, data, pixel_size } => {
                let rect = Rect::new(start_pos, size);
                RenderNode::Image { rect, data, pixel_size }
            }
            LayoutNode::Box { child, background, radius, margin, padding, on_click, on_hover } => {
                let p = padding.as_ref().map(|g| g.value() as i32).unwrap_or(0);
                let mt = margin.as_ref().map(|m| m.top() as i32).unwrap_or(0);
                let mb = margin.as_ref().map(|m| m.bottom() as i32).unwrap_or(0);
                let ml = margin.as_ref().map(|m| m.left() as i32).unwrap_or(0);
                let mr = margin.as_ref().map(|m| m.right() as i32).unwrap_or(0);

                let child_pos = Position::new(start_pos.x() + p + ml, start_pos.y() + p + mt);
                let render_child = Self::layout(*child, measurer, child_pos);
                let child_rect = render_child.rect();
                
                let box_size = Size::new(
                    (child_rect.width() as i32 + p * 2 + ml + mr) as u32,
                    (child_rect.height() as i32 + p * 2 + mt + mb) as u32
                );
                let rect = Rect::new(start_pos, box_size);
                
                RenderNode::Box {
                    rect,
                    child: Box::new(render_child),
                    background,
                    radius,
                    margin,
                    padding,
                    on_click,
                    on_hover,
                }
            }
            LayoutNode::Row { children, gap, align_items, on_click, on_hover } => {
                let mut current_x = start_pos.x();
                let mut max_h = 0;
                let gap_val = gap.map(|g| g.value() as i32).unwrap_or(0);
                
                let mut render_children = Vec::new();
                for child in children {
                    let child_pos = Position::new(current_x, start_pos.y());
                    let render_child = Self::layout(child, measurer, child_pos);
                    
                    let child_rect = render_child.rect();
                    current_x += child_rect.width() as i32 + gap_val;
                    if child_rect.height() > max_h {
                        max_h = child_rect.height();
                    }
                    render_children.push(render_child);
                }
                
                // Vertically align children
                for child in &mut render_children {
                    let child_h = child.rect().height();
                    if child_h < max_h {
                        let offset_y = match align_items {
                            AlignY::Top => 0,
                            AlignY::Center => ((max_h - child_h) / 2) as i32,
                            AlignY::Bottom => (max_h - child_h) as i32,
                        };
                        child.translate(0, offset_y);
                    }
                }
                
                let width = if current_x > start_pos.x() {
                    (current_x - start_pos.x() - gap_val) as u32
                } else {
                    0
                };
                
                let rect = Rect::new(start_pos, Size::new(width, max_h));
                
                RenderNode::Row {
                    rect,
                    children: render_children,
                    align_items,
                    on_click,
                    on_hover,
                }
            }
            LayoutNode::Stack { children, align_x, align_y, on_click, on_hover } => {
                let mut max_w = 0;
                let mut max_h = 0;
                
                let mut render_children = Vec::new();
                for child in children {
                    let render_child = Self::layout(child, measurer, start_pos);
                    let child_rect = render_child.rect();
                    if child_rect.width() > max_w { max_w = child_rect.width(); }
                    if child_rect.height() > max_h { max_h = child_rect.height(); }
                    render_children.push(render_child);
                }
                
                // Align children
                for child in &mut render_children {
                    let child_w = child.rect().width();
                    let child_h = child.rect().height();
                    let offset_x = if child_w < max_w {
                        match align_x {
                            AlignX::Left => 0,
                            AlignX::Center => ((max_w - child_w) / 2) as i32,
                            AlignX::Right => (max_w - child_w) as i32,
                        }
                    } else { 0 };
                    
                    let offset_y = if child_h < max_h {
                        match align_y {
                            AlignY::Top => 0,
                            AlignY::Center => ((max_h - child_h) / 2) as i32,
                            AlignY::Bottom => (max_h - child_h) as i32,
                        }
                    } else { 0 };
                    
                    child.translate(offset_x, offset_y);
                }
                
                let rect = Rect::new(start_pos, Size::new(max_w, max_h));
                
                RenderNode::Stack {
                    rect,
                    children: render_children,
                    align_x,
                    align_y,
                    on_click,
                    on_hover,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::shared::color::Color;

    struct MockTextMeasurer;

    impl TextMeasurer for MockTextMeasurer {
        fn measure(&mut self, text: &str, _font: Option<&FontFamily>, _size: Option<FontSize>) -> Size {
            Size::new(text.len() as u32 * 10, 20)
        }
    }

    #[test]
    fn test_layout_rect() {
        let node = LayoutNode::Rect {
            size: Size::new(50, 50),
            color: DrawingColor::Solid(Color::new(255, 0, 0, 255)),
            radius: None,
            on_click: None,
            on_hover: None,
        };
        let mut measurer = MockTextMeasurer;
        let start_pos = Position::new(10, 15);
        let render_node = LayoutEngine::layout(node, &mut measurer, start_pos);

        assert_eq!(render_node.rect().x(), 10);
        assert_eq!(render_node.rect().y(), 15);
        assert_eq!(render_node.rect().width(), 50);
        assert_eq!(render_node.rect().height(), 50);
    }

    #[test]
    fn test_layout_text() {
        let node = LayoutNode::Text {
            text: TextContent::new("Hello".to_string()),
            color: DrawingColor::Solid(Color::new(255, 255, 255, 255)),
            font: None,
            size: None,
            on_click: None,
            on_hover: None,
        };
        let mut measurer = MockTextMeasurer;
        let render_node = LayoutEngine::layout(node, &mut measurer, Position::new(0, 0));

        assert_eq!(render_node.rect().x(), 0);
        assert_eq!(render_node.rect().y(), 0);
        assert_eq!(render_node.rect().width(), 50); 
        assert_eq!(render_node.rect().height(), 20);
    }

    #[test]
    fn test_layout_row_with_gap() {
        let node = LayoutNode::Row {
            gap: Some(Gap(5.0)),
            align_items: AlignY::Top,
            on_click: None,
            on_hover: None,
            children: vec![
                LayoutNode::Rect {
                    size: Size::new(20, 30),
                    color: DrawingColor::Solid(Color::new(0, 0, 0, 255)),
                    radius: None,
                    on_click: None,
                    on_hover: None,
                },
                LayoutNode::Text {
                    text: TextContent::new("Hi".to_string()),
                    color: DrawingColor::Solid(Color::new(255, 255, 255, 255)),
                    font: None,
                    size: None,
                    on_click: None,
                    on_hover: None,
                },
            ],
        };
        
        let mut measurer = MockTextMeasurer;
        let render_node = LayoutEngine::layout(node, &mut measurer, Position::new(0, 0));

        assert_eq!(render_node.rect().width(), 45);
        assert_eq!(render_node.rect().height(), 30);
        
        if let RenderNode::Row { children, .. } = render_node {
            assert_eq!(children.len(), 2);
            assert_eq!(children[0].rect().x(), 0);
            assert_eq!(children[0].rect().width(), 20);
            
            assert_eq!(children[1].rect().x(), 25);
            assert_eq!(children[1].rect().width(), 20);
        } else {
            panic!("Expected Row");
        }
    }
    #[test]
    fn test_hit_test() {
        let node = LayoutNode::Row {
            gap: Some(Gap(10.0)),
            align_items: AlignY::Top,
            on_click: None,
            on_hover: None,
            children: vec![
                LayoutNode::Rect {
                    size: Size::new(20, 30),
                    color: DrawingColor::Solid(Color::new(0, 0, 0, 255)),
                    radius: None,
                    on_click: None,
                    on_hover: None,
                },
                LayoutNode::Text {
                    text: TextContent::new("ClickMe".to_string()), // size will be 7 * 10 = 70 width, 20 height
                    color: DrawingColor::Solid(Color::new(255, 255, 255, 255)),
                    font: None,
                    size: None,
                    on_click: Some(crate::domain::commands::AppCommand::AppletAction {
                        id: "test".to_string(),
                        action: "clicked".to_string(),
                    }),
                    on_hover: None,
                },
            ],
        };
        
        let mut measurer = MockTextMeasurer;
        let render_node = LayoutEngine::layout(node, &mut measurer, Position::new(50, 50));

        // Row rect: x=50, y=50, width=20 + 10 + 70 = 100, height=30
        
        // Miss completely
        assert_eq!(render_node.hit_test(Position::new(0, 0)), None);
        assert_eq!(render_node.hit_test(Position::new(150, 150)), None);

        // Hit Row but miss children (in the gap)
        // Rect 1: x=50..70, y=50..80
        // Gap: x=70..80
        // Text: x=80..150, y=50..70
        let gap_hit = render_node.hit_test(Position::new(75, 55)).unwrap();
        // It should hit the Row since gap is part of the row
        assert!(matches!(gap_hit, RenderNode::Row { .. }));

        // Hit Rect 1
        let rect_hit = render_node.hit_test(Position::new(60, 60)).unwrap();
        assert!(matches!(rect_hit, RenderNode::Rect { .. }));

        // Hit Text
        let text_hit = render_node.hit_test(Position::new(90, 60)).unwrap();
        assert!(matches!(text_hit, RenderNode::Text { .. }));
        
        assert!(text_hit.on_click().is_some());
        assert_eq!(*text_hit.on_click().unwrap(), crate::domain::commands::AppCommand::AppletAction { id: "test".to_string(), action: "clicked".to_string() });
    }
}
