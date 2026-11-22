mod canvas;
mod fonts;
mod glyphs;
mod measure;
mod pixels;
mod placement;
mod text;

use crate::shared::primitives::geometry::{Scale, Size};
use crate::shared::rendering::ports::draw::PaintTarget;

/// A target over all of `pixmap`, drawn into at `scale`.
fn target(pixmap: &mut tiny_skia::Pixmap, scale: f32) -> PaintTarget<'_> {
    let size = Size::new(pixmap.width(), pixmap.height());
    PaintTarget::new(pixmap.data_mut(), size, Scale::new(scale))
}

/// A canvas over all of `pixmap` at scale 1, with a 14px sans-serif default font.
fn sans_canvas<'a>(
    pixmap: &'a mut tiny_skia::Pixmap,
    fonts: &'a mut super::factory::FontCache,
) -> super::canvas::TinySkiaCosmicCanvas<'a> {
    use crate::shared::config::domain::{FontFamily, FontSize};
    let font = crate::shared::rendering::ports::font::Font::new(
        FontFamily::new("sans-serif".to_string()),
        FontSize::new(14.0),
    );
    super::canvas::TinySkiaCosmicCanvas::new(target(pixmap, 1.0), fonts, font)
}
