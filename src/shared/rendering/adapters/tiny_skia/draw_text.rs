use super::canvas::TinySkiaCosmicCanvas;
use super::paint::{get_family, get_paint};
use crate::shared::config::domain::{FontFamily, FontSize};
use crate::shared::primitives::color::DrawingColor;
use crate::shared::primitives::geometry::{LogicalPx, Position, Size};
use cosmic_text::{Attrs, Buffer, Metrics, Shaping, SwashContent};
use tiny_skia::{Rect, Transform};

impl TinySkiaCosmicCanvas<'_> {
    pub(crate) fn draw_text_impl(
        &mut self,
        text: &str,
        font_family: Option<&FontFamily>,
        font_size: Option<FontSize>,
        color: &DrawingColor,
        position: Position,
    ) {
        let Some(pixmap) = &mut self.pixmap else {
            return;
        };
        let size = font_size.unwrap_or(self.default_font_size).value();
        let family = font_family.unwrap_or(&self.default_font_family).as_str();
        let pos_x = f32::from(i16::try_from(position.x()).unwrap_or(0));
        let pos_y = f32::from(i16::try_from(position.y()).unwrap_or(0));
        let physical_x = LogicalPx::new(pos_x)
            .apply_scale(&self.scale)
            .value();
        let physical_y = LogicalPx::new(pos_y)
            .apply_scale(&self.scale)
            .value();

        let physical_size = LogicalPx::new(size).apply_scale(&self.scale).value();
        let metrics = Metrics::new(physical_size, physical_size * 1.0);
        let mut buffer = Buffer::new(self.font_system, metrics);
        let attrs = Attrs::new().family(get_family(family));

        buffer.set_text(text, &attrs, Shaping::Advanced, None);
        buffer.shape_until_scroll(self.font_system, false);

        for run in buffer.layout_runs() {
            for glyph in run.glyphs {
                let physical_glyph = glyph.physical((physical_x, physical_y + run.line_y), 1.0);

                if let Some(image) = self
                    .swash_cache
                    .get_image(self.font_system, physical_glyph.cache_key)
                    && image.content == SwashContent::Mask
                {
                    let rect_x = f32::from(i16::try_from(physical_glyph.x.saturating_add(image.placement.left)).unwrap_or(0));
                    let rect_y = f32::from(i16::try_from(physical_glyph.y.saturating_sub(image.placement.top)).unwrap_or(0));
                    let glyph_w = f32::from(u16::try_from(image.placement.width).unwrap_or(0));
                    let glyph_h = f32::from(u16::try_from(image.placement.height).unwrap_or(0));

                    if let Some(physical_rect) = Rect::from_xywh(rect_x, rect_y, glyph_w, glyph_h) {
                        let paint = get_paint(color, physical_rect);

                        if image.placement.width > 0
                            && image.placement.height > 0
                            && let Some(mut glyph_pixmap) =
                                tiny_skia::Pixmap::new(image.placement.width, image.placement.height)
                            && let Some(glyph_rect) = Rect::from_xywh(0.0, 0.0, glyph_w, glyph_h)
                        {
                            glyph_pixmap.fill_rect(glyph_rect, &paint, Transform::identity(), None);

                            for (pixel, &mask_alpha) in
                                glyph_pixmap.pixels_mut().iter_mut().zip(image.data.iter())
                            {
                                let scale_channel = |c: u8| -> u8 {
                                    let val = u32::from(c)
                                        .saturating_mul(u32::from(mask_alpha))
                                        .checked_div(255)
                                        .unwrap_or(0);
                                    u8::try_from(val).unwrap_or(0)
                                };
                                let a = scale_channel(pixel.alpha());
                                let r = scale_channel(pixel.red());
                                let g = scale_channel(pixel.green());
                                let b = scale_channel(pixel.blue());
                                if let Some(c) = tiny_skia::PremultipliedColorU8::from_rgba(r, g, b, a) {
                                    *pixel = c;
                                } else {
                                    *pixel = tiny_skia::PremultipliedColorU8::TRANSPARENT;
                                }
                            }

                            pixmap.draw_pixmap(
                                physical_glyph.x.saturating_add(image.placement.left),
                                physical_glyph.y.saturating_sub(image.placement.top),
                                glyph_pixmap.as_ref(),
                                &tiny_skia::PixmapPaint::default(),
                                Transform::identity(),
                                None,
                            );
                        }
                    }
                }
            }
        }
    }

    pub(crate) fn draw_image_impl(
        &mut self,
        image_data: &[u8],
        pixel_size: Size,
        logical_size: Size,
        position: Position,
    ) {
        let Some(pixmap) = &mut self.pixmap else {
            return;
        };
        let width = pixel_size.width();
        let height = pixel_size.height();
        let log_w = f32::from(u16::try_from(logical_size.width()).unwrap_or(0));
        let log_h = f32::from(u16::try_from(logical_size.height()).unwrap_or(0));
        let pos_x = f32::from(i16::try_from(position.x()).unwrap_or(0));
        let pos_y = f32::from(i16::try_from(position.y()).unwrap_or(0));
        let logical_width = LogicalPx::new(log_w);
        let logical_height = LogicalPx::new(log_h);
        let x = LogicalPx::new(pos_x);
        let y = LogicalPx::new(pos_y);
        let mut bgra_premul = Vec::with_capacity(image_data.len());
        for chunk in image_data.chunks_exact(4) {
            if let &[r, g, b, a] = chunk {
                let premul = |c: u8| -> u8 {
                    let val = u32::from(c)
                        .saturating_mul(u32::from(a))
                        .checked_div(255)
                        .unwrap_or(0);
                    u8::try_from(val).unwrap_or(0)
                };

                let r_p = premul(r);
                let g_p = premul(g);
                let b_p = premul(b);

                bgra_premul.push(b_p);
                bgra_premul.push(g_p);
                bgra_premul.push(r_p);
                bgra_premul.push(a);
            }
        }

        if let Some(image_pixmap) = tiny_skia::PixmapRef::from_bytes(&bgra_premul, width, height) {
            let paint = tiny_skia::PixmapPaint {
                quality: tiny_skia::FilterQuality::Bilinear,
                ..tiny_skia::PixmapPaint::default()
            };

            let physical_w = logical_width.apply_scale(&self.scale).value();
            let physical_h = logical_height.apply_scale(&self.scale).value();

            let px_w = f32::from(u16::try_from(width).unwrap_or(1));
            let px_h = f32::from(u16::try_from(height).unwrap_or(1));
            let scale_x = physical_w / px_w;
            let scale_y = physical_h / px_h;

            let physical_x = x.apply_scale(&self.scale).value();
            let physical_y = y.apply_scale(&self.scale).value();

            let transform =
                Transform::from_scale(scale_x, scale_y).post_translate(physical_x, physical_y);

            pixmap.draw_pixmap(0, 0, image_pixmap, &paint, transform, None);
        }
    }
}
