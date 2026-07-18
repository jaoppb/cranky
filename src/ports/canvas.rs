use crate::domain::shared::color::DrawingColor;

use crate::domain::shared::geometry::{LogicalPx, Position, Scale, Size};
use crate::domain::config::{FontFamily, FontSize};

pub trait CanvasFactory: Send + Sync {
    fn create_canvas<'a>(
        &'a mut self,
        data: &'a mut [u8],
        size: Size,
        scale: Scale,
        font_family: FontFamily,
        font_size: FontSize,
    ) -> impl Canvas + 'a;

    fn create_text_measurer<'a>(
        &'a mut self,
        scale: Scale,
        font_family: FontFamily,
        font_size: FontSize,
    ) -> impl crate::domain::layout::TextMeasurer + 'a;
}

#[cfg_attr(test, mockall::automock)]
#[allow(clippy::needless_lifetimes)]
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
        size: crate::domain::shared::geometry::Size,
        color: DrawingColor,
        radius: LogicalPx,
        border_size: LogicalPx,
    );



    /// Draw text at a position
    fn draw_text<'a>(
        &mut self,
        text: &str,
        font_family: Option<&'a FontFamily>,
        font_size: Option<FontSize>,
        color: DrawingColor,
        position: Position,
    );

    /// Draw an RGBA image
    fn draw_image(
        &mut self,
        image_data: &[u8],
        pixel_size: crate::domain::shared::geometry::Size,
        logical_size: crate::domain::shared::geometry::Size,
        position: Position,
    );
}


