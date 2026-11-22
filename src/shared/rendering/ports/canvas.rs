//! The drawing surface a render pipeline paints into, and the factory that makes one for each
//! buffer.
use super::draw::{BorderStroke, PaintTarget, RgbaImage, RoundedRect};
use super::font::Font;
use super::text::{TextPlacement, TextRun};
use crate::shared::primitives::color::DrawingColor;
use crate::shared::primitives::geometry::{Rect, Scale};
#[cfg(test)]
use crate::shared::{
    config::domain::{FontFamily, FontSize},
    primitives::TextAlignment,
    primitives::geometry::{Position, Size},
};

/// Makes the canvas and text measurer a render pipeline needs, from shared font state so text
/// measures as it draws.
pub trait CanvasFactory: Send + Sync + Clone {
    /// A canvas that paints into `target` with `font` as the default font.
    #[must_use]
    fn create_canvas<'a>(&'a mut self, target: PaintTarget<'a>, font: Font) -> impl Canvas + 'a;

    /// A measurer that sizes text with the fonts, scale and default font a canvas from this
    /// factory draws it with.
    #[must_use]
    fn create_text_measurer(
        &mut self,
        scale: Scale,
        font: Font,
    ) -> impl crate::features::layout_engine::domain::TextMeasurer + '_;
}

/// The drawing operations the paint step issues for each laid-out node, given in logical pixels.
pub trait Canvas: Send + Sync {
    /// Fill `shape` with `color`.
    fn draw_rect(&mut self, shape: RoundedRect, color: DrawingColor);

    /// Stroke the inside edge of `shape` with `stroke`.
    fn draw_border(&mut self, shape: RoundedRect, stroke: BorderStroke);

    /// Draw `run` starting at `placement.origin()`, shifted within
    /// `placement.alignment().available_width()` per its `align()`.
    fn draw_text(&mut self, run: TextRun<'_>, placement: TextPlacement);

    /// Draw `image` stretched over `area`.
    fn draw_image(&mut self, image: RgbaImage<'_>, area: Rect);
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
    fn draw_rect(&mut self, shape: RoundedRect, color: DrawingColor) {
        self.calls.push(CanvasCall::Rect {
            area: area_of(&shape),
            radius: shape.radius().value(),
            color,
        });
    }

    fn draw_border(&mut self, shape: RoundedRect, stroke: BorderStroke) {
        self.calls.push(CanvasCall::Border {
            area: area_of(&shape),
            radius: shape.radius().value(),
            width: stroke.width().value(),
            color: stroke.color().clone(),
        });
    }

    fn draw_text(&mut self, run: TextRun<'_>, placement: TextPlacement) {
        self.calls.push(CanvasCall::Text {
            text: run.text().to_string(),
            font_family: run.font().family().cloned(),
            font_size: run.font().size(),
            color: run.color().clone(),
            position: placement.origin(),
            alignment: placement.alignment(),
        });
    }

    fn draw_image(&mut self, image: RgbaImage<'_>, area: Rect) {
        self.calls.push(CanvasCall::Image {
            pixel_size: image.size(),
            logical_size: *area.size(),
            position: Position::new(area.x(), area.y()),
        });
    }
}

/// `[x, y, width, height]` of `shape`, as a [`CanvasCall`] records it.
#[cfg(test)]
const fn area_of(shape: &RoundedRect) -> [f32; 4] {
    [
        shape.x().value(),
        shape.y().value(),
        shape.width().value(),
        shape.height().value(),
    ]
}
