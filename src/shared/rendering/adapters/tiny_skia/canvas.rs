//! The canvas that paints into a borrowed pixel buffer with tiny-skia and cosmic-text.
use super::factory::FontCache;
use crate::shared::primitives::color::DrawingColor;
use crate::shared::primitives::geometry::{Rect, Scale};
use crate::shared::rendering::ports::canvas::Canvas;
use crate::shared::rendering::ports::draw::{BorderStroke, PaintTarget, RgbaImage, RoundedRect};
use crate::shared::rendering::ports::font::Font;
use crate::shared::rendering::ports::text::{TextPlacement, TextRun};
use tiny_skia::PixmapMut;

pub(crate) struct TinySkiaCosmicCanvas<'a> {
    pub(crate) pixmap: Option<PixmapMut<'a>>,
    pub(super) fonts: &'a mut FontCache,
    pub(crate) scale: Scale,
    pub(crate) default_font: Font,
}

impl<'a> TinySkiaCosmicCanvas<'a> {
    /// A canvas over `target`, shaping text with `fonts` and `default_font`; it draws nothing
    /// when `target`'s buffer does not hold its size.
    #[must_use]
    pub(super) fn new(
        target: PaintTarget<'a>,
        fonts: &'a mut FontCache,
        default_font: Font,
    ) -> Self {
        let size = target.size();
        let scale = target.scale();
        let pixmap = PixmapMut::from_bytes(target.into_pixels(), size.width(), size.height());
        Self {
            pixmap,
            fonts,
            scale,
            default_font,
        }
    }
}

impl Canvas for TinySkiaCosmicCanvas<'_> {
    fn draw_rect(&mut self, shape: RoundedRect, color: DrawingColor) {
        self.draw_rect_impl(shape, &color);
    }

    fn draw_border(&mut self, shape: RoundedRect, stroke: BorderStroke) {
        self.draw_border_impl(shape, &stroke);
    }

    fn draw_text(&mut self, run: TextRun<'_>, placement: TextPlacement) {
        self.draw_text_impl(&run, placement);
    }

    fn draw_image(&mut self, image: RgbaImage<'_>, area: Rect) {
        self.draw_image_impl(image, area);
    }
}
