use crate::shared::primitives::color::DrawingColor;

use crate::shared::config::domain::{FontFamily, FontSize};
use crate::shared::primitives::geometry::{LogicalPx, Position, Scale, Size};

pub trait CanvasFactory: Send + Sync {
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

#[cfg_attr(test, allow(clippy::ref_option_ref, clippy::struct_field_names))]
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
        size: Size,
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
        pixel_size: Size,
        logical_size: Size,
        position: Position,
    );
}
