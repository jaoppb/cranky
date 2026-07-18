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

    pub fn hit_test(&self, pos: crate::domain::shared::geometry::Position) -> Option<&RenderNode> {
        let r = self.rect();
        if pos.x() >= r.x() && pos.x() < r.x() + r.width() as i32 && pos.y() >= r.y() && pos.y() < r.y() + r.height() as i32 {
            if let Self::Flex { children, .. } = self {
                for child in children {
                    if let Some(hit) = child.hit_test(pos) {
                        return Some(hit);
                    }
                }
            }
            Some(self)
        } else {
            None
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
            Self::Image { rect, data, pixel_size } => {
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
