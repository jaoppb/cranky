use super::canvas::TinySkiaCosmicCanvas;
use super::measurer::CosmicTextMeasurer;
use crate::shared::config::domain::{FontFamily, FontSize};
use crate::shared::primitives::geometry::{Scale, Size};
use crate::shared::rendering::ports::canvas::{Canvas, CanvasFactory};
use cosmic_text::{FontSystem, SwashCache};
use tiny_skia::PixmapMut;

pub struct TinySkiaCanvasFactory {
    font_system: FontSystem,
    swash_cache: SwashCache,
}

impl Default for TinySkiaCanvasFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for TinySkiaCanvasFactory {
    fn clone(&self) -> Self {
        Self::new()
    }
}

impl TinySkiaCanvasFactory {
    #[must_use]
    pub fn new() -> Self {
        Self {
            font_system: FontSystem::new(),
            swash_cache: SwashCache::new(),
        }
    }
}

impl CanvasFactory for TinySkiaCanvasFactory {
    fn create_canvas<'a>(
        &'a mut self,
        data: &'a mut [u8],
        size: Size,
        scale: Scale,
        font_family: FontFamily,
        font_size: FontSize,
    ) -> impl Canvas + 'a {
        let pixmap = PixmapMut::from_bytes(data, size.width(), size.height());
        TinySkiaCosmicCanvas::from_optional_pixmap(
            pixmap,
            &mut self.font_system,
            &mut self.swash_cache,
            scale,
            font_family,
            font_size,
        )
    }

    fn create_text_measurer(
        &mut self,
        scale: Scale,
        font_family: FontFamily,
        font_size: FontSize,
    ) -> impl crate::features::layout_engine::domain::TextMeasurer + '_ {
        CosmicTextMeasurer::new(&mut self.font_system, scale, font_family, font_size)
    }
}
