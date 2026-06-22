use crate::domain::shared::color::DrawingColor;

use crate::domain::config::{FontFamily, FontSize};
use crate::domain::shared::geometry::{LogicalPx, Position};

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

    /// Measure text dimensions (width, height)
    fn measure_text<'a>(
        &mut self,
        text: &str,
        font_family: Option<&'a FontFamily>,
        font_size: Option<FontSize>,
    ) -> (LogicalPx, LogicalPx);

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
