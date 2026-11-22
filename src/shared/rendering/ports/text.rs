//! The text a canvas draws: a run of text in a font and color, and where it goes.
use crate::shared::config::domain::{FontFamily, FontSize};
use crate::shared::primitives::TextAlignment;
use crate::shared::primitives::color::DrawingColor;
use crate::shared::primitives::geometry::Position;

/// The font a text's style asks for; a part it leaves `None` falls back to the canvas default.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct TextFont<'a> {
    family: Option<&'a FontFamily>,
    size: Option<FontSize>,
}

impl<'a> TextFont<'a> {
    /// Asks for `family` and `size`, each when given.
    #[must_use]
    pub const fn new(family: Option<&'a FontFamily>, size: Option<FontSize>) -> Self {
        Self { family, size }
    }

    #[must_use]
    pub(crate) const fn family(&self) -> Option<&'a FontFamily> {
        self.family
    }

    #[must_use]
    pub(crate) const fn size(&self) -> Option<FontSize> {
        self.size
    }
}

/// A run of text to draw: the string, its font and its color.
#[derive(Debug, Clone, PartialEq)]
pub struct TextRun<'a> {
    text: &'a str,
    font: TextFont<'a>,
    color: DrawingColor,
}

impl<'a> TextRun<'a> {
    /// `text` in `font` and `color`.
    #[must_use]
    pub const fn new(text: &'a str, font: TextFont<'a>, color: DrawingColor) -> Self {
        Self { text, font, color }
    }

    #[must_use]
    pub(crate) const fn text(&self) -> &'a str {
        self.text
    }

    #[must_use]
    pub(crate) const fn font(&self) -> TextFont<'a> {
        self.font
    }

    #[must_use]
    pub(crate) const fn color(&self) -> &DrawingColor {
        &self.color
    }
}

/// Where a text run goes: the origin of its box, and how it aligns within the box's width.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextPlacement {
    origin: Position,
    alignment: TextAlignment,
}

impl TextPlacement {
    /// A box at `origin`, the text shifted in it per `alignment`.
    #[must_use]
    pub const fn new(origin: Position, alignment: TextAlignment) -> Self {
        Self { origin, alignment }
    }

    #[must_use]
    pub(crate) const fn origin(&self) -> Position {
        self.origin
    }

    #[must_use]
    pub(crate) const fn alignment(&self) -> TextAlignment {
        self.alignment
    }
}
