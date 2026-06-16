use crate::ports::DisplayServerError;
use crate::domain::color::DrawingColor;

use crate::domain::config::{FontFamily, FontSize};
use crate::domain::geometry::Position;

#[cfg_attr(test, mockall::automock)]
pub trait Canvas: Send + Sync {
    /// Clear the canvas with transparent pixels
    fn clear(&mut self);

    /// Draw a filled rectangle with optional radius
    fn draw_rect(&mut self, x: f32, y: f32, width: f32, height: f32, color: DrawingColor, radius: f32);

    /// Draw a border around a rectangle
    fn draw_border(&mut self, x: f32, y: f32, width: f32, height: f32, color: DrawingColor, radius: f32, size: f32);

    /// Measure text dimensions (width, height)
    fn measure_text<'a>(&mut self, text: &str, font_family: Option<&'a FontFamily>, font_size: Option<FontSize>) -> (f32, f32);

    /// Draw text at a position
    fn draw_text<'a>(&mut self, text: &str, font_family: Option<&'a FontFamily>, font_size: Option<FontSize>, color: DrawingColor, position: Position);

    /// Draw an RGBA image
    fn draw_image(
        &mut self,
        image_data: &[crate::domain::color::Color],
        width: u32,
        height: u32,
        logical_width: f32,
        logical_height: f32,
        x: f32,
        y: f32,
    );

    /// Finalize rendering
    fn flush(&mut self) -> Result<(), DisplayServerError>;
}
