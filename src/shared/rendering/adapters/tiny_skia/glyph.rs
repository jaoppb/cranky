//! A rasterized glyph placed on the canvas, tinted with the text's paint and composited.
use super::paint::get_paint;
use crate::shared::primitives::color::DrawingColor;
use cosmic_text::{PhysicalGlyph, SwashImage};
use tiny_skia::{Paint, Pixmap, PixmapMut, PixmapPaint, Rect, Transform};

/// A glyph's coverage image and the canvas pixel its top-left corner lands on.
pub(super) struct PlacedGlyph<'image> {
    image: &'image SwashImage,
    x: i32,
    y: i32,
}

impl<'image> PlacedGlyph<'image> {
    /// `image` placed for `glyph`: the glyph's pen position moved by the image's own offset.
    pub(super) const fn new(image: &'image SwashImage, glyph: &PhysicalGlyph) -> Self {
        Self {
            image,
            x: glyph.x.saturating_add(image.placement.left),
            y: glyph.y.saturating_sub(image.placement.top),
        }
    }

    /// Composites the glyph onto `pixmap`, painted with `color` across the glyph's own box.
    pub(super) fn draw(&self, pixmap: &mut PixmapMut<'_>, color: &DrawingColor) {
        let rect_x = f32::from(i16::try_from(self.x).unwrap_or(0));
        let rect_y = f32::from(i16::try_from(self.y).unwrap_or(0));
        let (glyph_w, glyph_h) = self.size();
        let Some(physical_rect) = Rect::from_xywh(rect_x, rect_y, glyph_w, glyph_h) else {
            return;
        };
        let paint = get_paint(color, physical_rect);
        if let Some(glyph_pixmap) = self.tinted(&paint) {
            pixmap.draw_pixmap(
                self.x,
                self.y,
                glyph_pixmap.as_ref(),
                &PixmapPaint::default(),
                Transform::identity(),
                None,
            );
        }
    }

    /// The glyph's width and height in pixels (0 when one does not fit a `u16`).
    fn size(&self) -> (f32, f32) {
        let placement = self.image.placement;
        (
            f32::from(u16::try_from(placement.width).unwrap_or(0)),
            f32::from(u16::try_from(placement.height).unwrap_or(0)),
        )
    }

    /// A pixmap of the glyph's size filled with `paint` and masked by its coverage, or `None`
    /// for a glyph without pixels.
    fn tinted(&self, paint: &Paint<'_>) -> Option<Pixmap> {
        let placement = self.image.placement;
        if placement.width == 0 || placement.height == 0 {
            return None;
        }
        let (glyph_w, glyph_h) = self.size();
        let mut glyph_pixmap = Pixmap::new(placement.width, placement.height)?;
        let glyph_rect = Rect::from_xywh(0.0, 0.0, glyph_w, glyph_h)?;
        glyph_pixmap.fill_rect(glyph_rect, paint, Transform::identity(), None);
        apply_glyph_mask(&mut glyph_pixmap, &self.image.data);
        Some(glyph_pixmap)
    }
}

/// Scales every pixel of `glyph_pixmap` (filled with the text's paint) by the coverage `mask`
/// cosmic-text rasterized for the glyph, leaving a premultiplied glyph ready to composite.
fn apply_glyph_mask(glyph_pixmap: &mut Pixmap, mask: &[u8]) {
    for (pixel, &mask_alpha) in glyph_pixmap.pixels_mut().iter_mut().zip(mask.iter()) {
        let scale_channel = |channel: u8| -> u8 {
            let val = u32::from(channel)
                .saturating_mul(u32::from(mask_alpha))
                .checked_div(255)
                .unwrap_or(0);
            u8::try_from(val).unwrap_or(0)
        };
        let alpha = scale_channel(pixel.alpha());
        let red = scale_channel(pixel.red());
        let green = scale_channel(pixel.green());
        let blue = scale_channel(pixel.blue());
        if let Some(premultiplied) =
            tiny_skia::PremultipliedColorU8::from_rgba(red, green, blue, alpha)
        {
            *pixel = premultiplied;
        } else {
            *pixel = tiny_skia::PremultipliedColorU8::TRANSPARENT;
        }
    }
}
