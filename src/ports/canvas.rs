use crate::ports::DisplayServerError;
use crate::domain::shared::color::DrawingColor;

use crate::domain::config::{FontFamily, FontSize};
use crate::domain::shared::geometry::{Position, LogicalPx};

#[cfg_attr(test, mockall::automock)]
pub trait Canvas: Send + Sync {
    /// Clear the canvas with transparent pixels
    fn clear(&mut self);

    /// Draw a filled rectangle with optional radius
    fn draw_rect(&mut self, x: LogicalPx, y: LogicalPx, width: LogicalPx, height: LogicalPx, color: DrawingColor, radius: LogicalPx);

    /// Draw a border around a rectangle
    fn draw_border(&mut self, x: LogicalPx, y: LogicalPx, width: LogicalPx, height: LogicalPx, color: DrawingColor, radius: LogicalPx, size: LogicalPx);

    /// Measure text dimensions (width, height)
    fn measure_text<'a>(&mut self, text: &str, font_family: Option<&'a FontFamily>, font_size: Option<FontSize>) -> (LogicalPx, LogicalPx);

    /// Draw text at a position
    fn draw_text<'a>(&mut self, text: &str, font_family: Option<&'a FontFamily>, font_size: Option<FontSize>, color: DrawingColor, position: Position);

    /// Draw an RGBA image
    fn draw_image(
        &mut self,
        image_data: &[u8],
        width: u32,
        height: u32,
        logical_width: LogicalPx,
        logical_height: LogicalPx,
        x: LogicalPx,
        y: LogicalPx,
    );

    /// Finalize rendering
    fn flush(&mut self) -> Result<(), DisplayServerError>;
}
