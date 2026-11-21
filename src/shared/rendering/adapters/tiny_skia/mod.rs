//! The production renderer: tiny-skia paints shapes and images, cosmic-text shapes and draws text.
pub mod canvas;
mod draw_rect;
mod draw_text;
mod factory;
pub mod measurer;
mod paint;
#[cfg(test)]
mod tests;

pub use factory::TinySkiaCanvasFactory;
