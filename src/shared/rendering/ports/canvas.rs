use crate::shared::primitives::color::DrawingColor;

use crate::shared::config::domain::{FontFamily, FontSize};
use crate::shared::primitives::TextAlignment;
use crate::shared::primitives::geometry::{LogicalPx, Position, Scale, Size};

pub trait CanvasFactory: Send + Sync + Clone {
    #[must_use]
    fn create_canvas<'a>(
        &'a mut self,
        data: &'a mut [u8],
        size: Size,
        scale: Scale,
        font_family: FontFamily,
        font_size: FontSize,
    ) -> impl Canvas + 'a;

    #[must_use]
    fn create_text_measurer(
        &mut self,
        scale: Scale,
        font_family: FontFamily,
        font_size: FontSize,
    ) -> impl crate::features::layout_engine::domain::TextMeasurer + '_;
}

pub trait Canvas: Send + Sync {
    /// Draw a filled rectangle with optional radius
    fn draw_rect(
        &mut self,
        x: LogicalPx,
        y: LogicalPx,
        width: LogicalPx,
        height: LogicalPx,
        color: DrawingColor,
        radius: LogicalPx,
    );

    /// Draw a border around a rectangle
    fn draw_border(
        &mut self,
        position: Position,
        size: Size,
        color: DrawingColor,
        radius: LogicalPx,
        border_size: LogicalPx,
    );

    /// Draw text starting at `position`, shifted within
    /// `alignment.available_width()` per `alignment.align()`.
    fn draw_text(
        &mut self,
        text: &str,
        font_family: Option<&FontFamily>,
        font_size: Option<FontSize>,
        color: DrawingColor,
        position: Position,
        alignment: TextAlignment,
    );

    /// Draw an RGBA image
    fn draw_image(
        &mut self,
        image_data: &[u8],
        pixel_size: Size,
        logical_size: Size,
        position: Position,
    );
}

/// One request a [`RecordingCanvas`] received.
#[cfg(test)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CanvasCall {
    Rect,
    Border,
    Text {
        text: String,
        position: Position,
        alignment: TextAlignment,
    },
    Image,
}

#[cfg(test)]
impl CanvasCall {
    /// The request's name, for asserting on which requests were made.
    #[must_use]
    pub(crate) const fn kind(&self) -> &'static str {
        match self {
            Self::Rect => "rect",
            Self::Border => "border",
            Self::Text { .. } => "text",
            Self::Image => "image",
        }
    }
}

/// A canvas that draws nothing and remembers every request, in order.
#[cfg(test)]
#[derive(Debug, Default)]
pub(crate) struct RecordingCanvas {
    pub(crate) calls: Vec<CanvasCall>,
}

#[cfg(test)]
impl RecordingCanvas {
    /// The name of every request received, in order.
    #[must_use]
    pub(crate) fn kinds(&self) -> Vec<&'static str> {
        self.calls.iter().map(CanvasCall::kind).collect()
    }
}

#[cfg(test)]
impl Canvas for RecordingCanvas {
    fn draw_rect(
        &mut self,
        _x: LogicalPx,
        _y: LogicalPx,
        _width: LogicalPx,
        _height: LogicalPx,
        _color: DrawingColor,
        _radius: LogicalPx,
    ) {
        self.calls.push(CanvasCall::Rect);
    }

    fn draw_border(
        &mut self,
        _position: Position,
        _size: Size,
        _color: DrawingColor,
        _radius: LogicalPx,
        _border_size: LogicalPx,
    ) {
        self.calls.push(CanvasCall::Border);
    }

    fn draw_text(
        &mut self,
        text: &str,
        _font_family: Option<&FontFamily>,
        _font_size: Option<FontSize>,
        _color: DrawingColor,
        position: Position,
        alignment: TextAlignment,
    ) {
        self.calls.push(CanvasCall::Text {
            text: text.to_string(),
            position,
            alignment,
        });
    }

    fn draw_image(
        &mut self,
        _image_data: &[u8],
        _pixel_size: Size,
        _logical_size: Size,
        _position: Position,
    ) {
        self.calls.push(CanvasCall::Image);
    }
}
