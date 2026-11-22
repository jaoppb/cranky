//! The drawing surface a render pipeline paints into, and the factory that makes one for each
//! buffer.
use crate::shared::primitives::color::DrawingColor;

use crate::shared::config::domain::{FontFamily, FontSize};
use crate::shared::primitives::TextAlignment;
use crate::shared::primitives::geometry::{LogicalPx, Position, Scale, Size};

/// Makes the canvas and text measurer a render pipeline needs, from shared font state so text
/// measures as it draws.
pub trait CanvasFactory: Send + Sync + Clone {
    /// A canvas that paints into `data`, an RGBA buffer of `size` physical pixels, at `scale` with
    /// the given default font.
    #[must_use]
    fn create_canvas<'a>(
        &'a mut self,
        data: &'a mut [u8],
        size: Size,
        scale: Scale,
        font_family: FontFamily,
        font_size: FontSize,
    ) -> impl Canvas + 'a;

    /// A measurer that sizes text with the fonts, scale and defaults a canvas from this factory
    /// draws it with.
    #[must_use]
    fn create_text_measurer(
        &mut self,
        scale: Scale,
        font_family: FontFamily,
        font_size: FontSize,
    ) -> impl crate::features::layout_engine::domain::TextMeasurer + '_;
}

/// The drawing operations the paint step issues for each laid-out node, given in logical pixels.
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

/// One request a [`RecordingCanvas`] received, with the values it was given; areas are
/// `[x, y, width, height]` in logical pixels.
#[cfg(test)]
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum CanvasCall {
    Rect {
        area: [f32; 4],
        radius: f32,
        color: DrawingColor,
    },
    Border {
        area: [f32; 4],
        radius: f32,
        width: f32,
        color: DrawingColor,
    },
    Text {
        text: String,
        font_family: Option<FontFamily>,
        font_size: Option<FontSize>,
        color: DrawingColor,
        position: Position,
        alignment: TextAlignment,
    },
    Image {
        pixel_size: Size,
        logical_size: Size,
        position: Position,
    },
}

#[cfg(test)]
impl CanvasCall {
    /// The request's name, for asserting on which requests were made.
    #[must_use]
    pub(crate) const fn kind(&self) -> &'static str {
        match self {
            Self::Rect { .. } => "rect",
            Self::Border { .. } => "border",
            Self::Text { .. } => "text",
            Self::Image { .. } => "image",
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
        x: LogicalPx,
        y: LogicalPx,
        width: LogicalPx,
        height: LogicalPx,
        color: DrawingColor,
        radius: LogicalPx,
    ) {
        self.calls.push(CanvasCall::Rect {
            area: [x.value(), y.value(), width.value(), height.value()],
            radius: radius.value(),
            color,
        });
    }

    fn draw_border(
        &mut self,
        position: Position,
        size: Size,
        color: DrawingColor,
        radius: LogicalPx,
        border_size: LogicalPx,
    ) {
        self.calls.push(CanvasCall::Border {
            area: [
                f32::from(i16::try_from(position.x()).unwrap_or(0)),
                f32::from(i16::try_from(position.y()).unwrap_or(0)),
                f32::from(u16::try_from(size.width()).unwrap_or(0)),
                f32::from(u16::try_from(size.height()).unwrap_or(0)),
            ],
            radius: radius.value(),
            width: border_size.value(),
            color,
        });
    }

    fn draw_text(
        &mut self,
        text: &str,
        font_family: Option<&FontFamily>,
        font_size: Option<FontSize>,
        color: DrawingColor,
        position: Position,
        alignment: TextAlignment,
    ) {
        self.calls.push(CanvasCall::Text {
            text: text.to_string(),
            font_family: font_family.cloned(),
            font_size,
            color,
            position,
            alignment,
        });
    }

    fn draw_image(
        &mut self,
        _image_data: &[u8],
        pixel_size: Size,
        logical_size: Size,
        position: Position,
    ) {
        self.calls.push(CanvasCall::Image {
            pixel_size,
            logical_size,
            position,
        });
    }
}
