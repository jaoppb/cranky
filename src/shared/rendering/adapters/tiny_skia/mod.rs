//! The production renderer: tiny-skia paints shapes and images, cosmic-text shapes and draws text.
pub mod canvas;
mod draw_rect;
mod draw_text;
mod factory;
mod glyph;
pub mod measurer;
mod paint;
mod physical_box;
mod shaping;
#[cfg(test)]
mod tests;

pub use factory::TinySkiaCanvasFactory;
