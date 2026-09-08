use crate::shared::primitives::color::{Color, DrawingColor};
use crate::shared::primitives::PopupOffset;
use serde::{Deserialize, Serialize};

use super::types::{BorderRadius, BorderSize, FontFamily, FontSize, PaddingOffset};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PopupBehavior {
    PerMonitor,
    PerModule,
    PerModuleAndMonitor,
    #[default]
    Global,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PopupConfig {
    behavior: PopupBehavior,
    offset: PopupOffset,
}

impl Default for PopupConfig {
    fn default() -> Self {
        Self {
            behavior: PopupBehavior::default(),
            offset: PopupOffset::new(0, 8),
        }
    }
}

impl PopupConfig {
    #[must_use]
    pub const fn new(behavior: PopupBehavior, offset: PopupOffset) -> Self {
        Self { behavior, offset }
    }

    #[must_use]
    pub const fn behavior(&self) -> PopupBehavior {
        self.behavior
    }

    #[must_use]
    pub const fn offset(&self) -> PopupOffset {
        self.offset
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TooltipConfig {
    background: DrawingColor,
    border_color: DrawingColor,
    text_color: DrawingColor,
    font: Option<FontFamily>,
    size: Option<FontSize>,
    radius: BorderRadius,
    border_width: BorderSize,
    padding: PaddingOffset,
}

impl TooltipConfig {
    #[must_use]
    pub fn with_background(mut self, background: DrawingColor) -> Self {
        self.background = background;
        self
    }

    #[must_use]
    pub fn with_border_color(mut self, border_color: DrawingColor) -> Self {
        self.border_color = border_color;
        self
    }

    #[must_use]
    pub fn with_text_color(mut self, text_color: DrawingColor) -> Self {
        self.text_color = text_color;
        self
    }

    #[must_use]
    pub fn with_font(mut self, font: Option<FontFamily>) -> Self {
        self.font = font;
        self
    }

    #[must_use]
    pub const fn with_size(mut self, size: Option<FontSize>) -> Self {
        self.size = size;
        self
    }

    #[must_use]
    pub const fn with_radius(mut self, radius: BorderRadius) -> Self {
        self.radius = radius;
        self
    }

    #[must_use]
    pub const fn with_border_width(mut self, border_width: BorderSize) -> Self {
        self.border_width = border_width;
        self
    }

    #[must_use]
    pub const fn with_padding(mut self, padding: PaddingOffset) -> Self {
        self.padding = padding;
        self
    }

    #[must_use]
    pub const fn background(&self) -> &DrawingColor {
        &self.background
    }
    #[must_use]
    pub const fn border_color(&self) -> &DrawingColor {
        &self.border_color
    }
    #[must_use]
    pub const fn text_color(&self) -> &DrawingColor {
        &self.text_color
    }
    #[must_use]
    pub const fn font(&self) -> Option<&FontFamily> {
        self.font.as_ref()
    }
    #[must_use]
    pub const fn size(&self) -> Option<FontSize> {
        self.size
    }
    #[must_use]
    pub const fn radius(&self) -> BorderRadius {
        self.radius
    }
    #[must_use]
    pub const fn border_width(&self) -> BorderSize {
        self.border_width
    }
    #[must_use]
    pub const fn padding(&self) -> PaddingOffset {
        self.padding
    }
}

impl Default for TooltipConfig {
    fn default() -> Self {
        Self {
            background: DrawingColor::Solid(Color::new(0x1e, 0x1e, 0x2e, 255)),
            border_color: DrawingColor::Solid(Color::new(0xc0, 0xca, 0xf5, 255)),
            text_color: DrawingColor::Solid(Color::new(0xc0, 0xca, 0xf5, 255)),
            font: Some(FontFamily::new("Inter".to_string())),
            size: Some(FontSize::new(12.0)),
            radius: BorderRadius::new(4.0),
            border_width: BorderSize::new(1.0),
            padding: PaddingOffset::new(8),
        }
    }
}
