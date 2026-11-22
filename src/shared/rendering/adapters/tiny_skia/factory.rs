use super::canvas::TinySkiaCosmicCanvas;
use super::measurer::CosmicTextMeasurer;
use crate::shared::primitives::geometry::Scale;
use crate::shared::rendering::ports::canvas::{Canvas, CanvasFactory};
use crate::shared::rendering::ports::draw::PaintTarget;
use crate::shared::rendering::ports::font::Font;
use cosmic_text::{FontSystem, SwashCache};

/// The font system text is shaped with and the cache its glyphs are rasterised into, which the
/// canvases of one factory share.
#[derive(Debug)]
pub(super) struct FontCache {
    pub(super) font_system: FontSystem,
    pub(super) swash_cache: SwashCache,
}

impl FontCache {
    /// Loads the system fonts into a new font system, with an empty glyph cache.
    #[must_use]
    pub(super) fn new() -> Self {
        Self {
            font_system: FontSystem::new(),
            swash_cache: SwashCache::new(),
        }
    }
}

/// Makes the canvases and text measurers a render pipeline paints with, all sharing its one font
/// system and glyph cache.
///
/// A clone loads its own font system and cache rather than sharing them.
#[derive(Debug)]
pub struct TinySkiaCanvasFactory {
    fonts: FontCache,
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
    /// Loads the system fonts into a new font system, with an empty glyph cache.
    #[must_use]
    pub fn new() -> Self {
        Self {
            fonts: FontCache::new(),
        }
    }
}

impl CanvasFactory for TinySkiaCanvasFactory {
    fn create_canvas<'a>(&'a mut self, target: PaintTarget<'a>, font: Font) -> impl Canvas + 'a {
        TinySkiaCosmicCanvas::new(target, &mut self.fonts, font)
    }

    fn create_text_measurer(
        &mut self,
        scale: Scale,
        font: Font,
    ) -> impl crate::features::layout_engine::domain::TextMeasurer + '_ {
        CosmicTextMeasurer::new(&mut self.fonts.font_system, scale, font)
    }
}
