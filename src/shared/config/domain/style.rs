//! Settings of the `[popup]` and `[tooltip]` tables.
use crate::shared::primitives::PopupOffset;
use crate::shared::primitives::color::{Color, DrawingColor};
use serde::{Deserialize, Serialize};

use super::types::{BorderRadius, BorderSize, FontFamily, FontSize, PaddingOffset};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PopupBehavior {
    PerMonitor,
    PerModule,
    PerModuleAndMonitor,
    #[default]
    Global,
}

/// How a module embedded in a popup is rendered: painted into the popup's own
/// buffer (`Composite`) or as a separate Wayland subsurface (`Subsurface`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum PopupEmbed {
    #[default]
    Composite,
    Subsurface,
}

/// How popups are grouped across monitors and modules, where they are offset from their
/// anchor, and whether embedded modules are composited or get their own subsurface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PopupConfig {
    behavior: PopupBehavior,
    offset: PopupOffset,
    embed: PopupEmbed,
}

impl Default for PopupConfig {
    fn default() -> Self {
        Self {
            behavior: PopupBehavior::default(),
            offset: PopupOffset::new(0, 8),
            embed: PopupEmbed::default(),
        }
    }
}

impl PopupConfig {
    #[must_use]
    pub(crate) const fn new(
        behavior: PopupBehavior,
        offset: PopupOffset,
        embed: PopupEmbed,
    ) -> Self {
        Self {
            behavior,
            offset,
            embed,
        }
    }

    #[must_use]
    pub(crate) const fn behavior(&self) -> PopupBehavior {
        self.behavior
    }

    #[must_use]
    pub(crate) const fn offset(&self) -> PopupOffset {
        self.offset
    }

    #[must_use]
    pub(crate) const fn embed(&self) -> PopupEmbed {
        self.embed
    }
}

/// Colors, font, border and padding parsed from the `[tooltip]` table.
/// Only tests read these fields back so far.
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
    pub(crate) fn with_background(mut self, background: DrawingColor) -> Self {
        self.background = background;
        self
    }

    #[must_use]
    pub(crate) fn with_border_color(mut self, border_color: DrawingColor) -> Self {
        self.border_color = border_color;
        self
    }

    #[must_use]
    pub(crate) fn with_text_color(mut self, text_color: DrawingColor) -> Self {
        self.text_color = text_color;
        self
    }

    #[must_use]
    pub(crate) fn with_font(mut self, font: Option<FontFamily>) -> Self {
        self.font = font;
        self
    }

    #[must_use]
    pub(crate) const fn with_size(mut self, size: Option<FontSize>) -> Self {
        self.size = size;
        self
    }

    #[must_use]
    pub(crate) const fn with_radius(mut self, radius: BorderRadius) -> Self {
        self.radius = radius;
        self
    }

    #[must_use]
    pub(crate) const fn with_border_width(mut self, border_width: BorderSize) -> Self {
        self.border_width = border_width;
        self
    }

    #[must_use]
    pub(crate) const fn with_padding(mut self, padding: PaddingOffset) -> Self {
        self.padding = padding;
        self
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) const fn background(&self) -> &DrawingColor {
        &self.background
    }
    #[cfg(test)]
    #[must_use]
    pub(crate) const fn border_color(&self) -> &DrawingColor {
        &self.border_color
    }
    #[cfg(test)]
    #[must_use]
    pub(crate) const fn text_color(&self) -> &DrawingColor {
        &self.text_color
    }
    #[cfg(test)]
    #[must_use]
    pub(crate) const fn font(&self) -> Option<&FontFamily> {
        self.font.as_ref()
    }
    #[cfg(test)]
    #[must_use]
    pub(crate) const fn size(&self) -> Option<FontSize> {
        self.size
    }
    #[cfg(test)]
    #[must_use]
    pub(crate) const fn radius(&self) -> BorderRadius {
        self.radius
    }
    #[cfg(test)]
    #[must_use]
    pub(crate) const fn border_width(&self) -> BorderSize {
        self.border_width
    }
    #[cfg(test)]
    #[must_use]
    pub(crate) const fn padding(&self) -> PaddingOffset {
        self.padding
    }
}

impl Default for TooltipConfig {
    fn default() -> Self {
        Self {
            background: DrawingColor::Solid(Color::rgb(0x1e, 0x1e, 0x2e)),
            border_color: DrawingColor::Solid(Color::rgb(0xc0, 0xca, 0xf5)),
            text_color: DrawingColor::Solid(Color::rgb(0xc0, 0xca, 0xf5)),
            font: Some(FontFamily::new("Inter".to_string())),
            size: Some(FontSize::new(12.0)),
            radius: BorderRadius::new(4.0),
            border_width: BorderSize::new(1.0),
            padding: PaddingOffset::new(8),
        }
    }
}
