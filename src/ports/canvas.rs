use crate::domain::errors::PortError;
use crate::domain::color::DrawingColor;

pub trait Canvas: Send + Sync {
    /// Draw a filled rectangle with optional radius
    fn draw_rect(&mut self, x: f32, y: f32, width: f32, height: f32, color: DrawingColor, radius: f32);

    /// Draw a border around a rectangle
    fn draw_border(&mut self, x: f32, y: f32, width: f32, height: f32, color: DrawingColor, radius: f32, size: f32);

    /// Measure text dimensions (width, height)
    fn measure_text(&mut self, text: &str, font_family: &str, font_size: f32) -> (f32, f32);

    /// Draw text at a position
    fn draw_text(&mut self, text: &str, font_family: &str, font_size: f32, color: DrawingColor, x: f32, y: f32);

    /// Finalize rendering
    fn flush(&mut self) -> Result<(), PortError>;
}
