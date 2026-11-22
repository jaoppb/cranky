use super::canvas::TinySkiaCosmicCanvas;
use super::paint::{get_family, get_paint};
use crate::shared::primitives::geometry::{LogicalPx, Rect as LogicalRect};
use crate::shared::rendering::ports::draw::RgbaImage;
use crate::shared::rendering::ports::text::{TextPlacement, TextRun};
use cosmic_text::{Attrs, Buffer, Metrics, Shaping, SwashContent};
use tiny_skia::{Rect, Transform};

impl TinySkiaCosmicCanvas<'_> {
    pub(crate) fn draw_text_impl(&mut self, run: &TextRun<'_>, placement: TextPlacement) {
        let Some(pixmap) = &mut self.pixmap else {
            return;
        };
        let text = run.text();
        let color = run.color();
        let alignment = placement.alignment();
        let position = placement.origin();
        let font = run.font();
        let size = font
            .size()
            .unwrap_or_else(|| self.default_font.size())
            .value();
        let family = font
            .family()
            .unwrap_or_else(|| self.default_font.family())
            .as_str();
        let pos_x = f32::from(i16::try_from(position.x()).unwrap_or(0));
        let pos_y = f32::from(i16::try_from(position.y()).unwrap_or(0));
        let origin_x = LogicalPx::new(pos_x).apply_scale(self.scale).value();
        let physical_y = LogicalPx::new(pos_y).apply_scale(self.scale).value();

        let physical_size = LogicalPx::new(size).apply_scale(self.scale).value();
        let metrics = Metrics::new(physical_size, physical_size * 1.0);
        let mut buffer = Buffer::new(&mut self.fonts.font_system, metrics);
        let attrs = Attrs::new().family(get_family(family));

        buffer.set_text(text, &attrs, Shaping::Advanced, None);
        buffer.shape_until_scroll(&mut self.fonts.font_system, false);

        let text_width = buffer
            .layout_runs()
            .fold(0.0_f32, |widest, run| widest.max(run.line_w));
        let available = LogicalPx::new(f32::from(
            u16::try_from(alignment.available_width()).unwrap_or(u16::MAX),
        ))
        .apply_scale(self.scale)
        .value();
        let physical_x = origin_x + alignment.align().offset(available, text_width);

        for run in buffer.layout_runs() {
            for glyph in run.glyphs {
                let physical_glyph = glyph.physical((physical_x, physical_y + run.line_y), 1.0);

                if let Some(image) = self
                    .fonts
                    .swash_cache
                    .get_image(&mut self.fonts.font_system, physical_glyph.cache_key)
                    && image.content == SwashContent::Mask
                {
                    let rect_x = f32::from(
                        i16::try_from(physical_glyph.x.saturating_add(image.placement.left))
                            .unwrap_or(0),
                    );
                    let rect_y = f32::from(
                        i16::try_from(physical_glyph.y.saturating_sub(image.placement.top))
                            .unwrap_or(0),
                    );
                    let glyph_w = f32::from(u16::try_from(image.placement.width).unwrap_or(0));
                    let glyph_h = f32::from(u16::try_from(image.placement.height).unwrap_or(0));

                    if let Some(physical_rect) = Rect::from_xywh(rect_x, rect_y, glyph_w, glyph_h) {
                        let paint = get_paint(color, physical_rect);

                        if image.placement.width > 0
                            && image.placement.height > 0
                            && let Some(mut glyph_pixmap) = tiny_skia::Pixmap::new(
                                image.placement.width,
                                image.placement.height,
                            )
                            && let Some(glyph_rect) = Rect::from_xywh(0.0, 0.0, glyph_w, glyph_h)
                        {
                            glyph_pixmap.fill_rect(glyph_rect, &paint, Transform::identity(), None);

                            apply_glyph_mask(&mut glyph_pixmap, &image.data);

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

    pub(crate) fn draw_image_impl(&mut self, image: RgbaImage<'_>, area: LogicalRect) {
        let Some(pixmap) = &mut self.pixmap else {
            return;
        };
        let image_data = image.data();
        let width = image.size().width();
        let height = image.size().height();
        let log_w = f32::from(u16::try_from(area.width()).unwrap_or(0));
        let log_h = f32::from(u16::try_from(area.height()).unwrap_or(0));
        let pos_x = f32::from(i16::try_from(area.x()).unwrap_or(0));
        let pos_y = f32::from(i16::try_from(area.y()).unwrap_or(0));
        let logical_width = LogicalPx::new(log_w);
        let logical_height = LogicalPx::new(log_h);
        let x = LogicalPx::new(pos_x);
        let y = LogicalPx::new(pos_y);
        let mut bgra_premul = Vec::with_capacity(image_data.len());
        for &[red, green, blue, alpha] in image_data.as_chunks::<4>().0 {
            let premul = |channel: u8| -> u8 {
                let val = u32::from(channel)
                    .saturating_mul(u32::from(alpha))
                    .checked_div(255)
                    .unwrap_or(0);
                u8::try_from(val).unwrap_or(0)
            };

            let r_p = premul(red);
            let g_p = premul(green);
            let b_p = premul(blue);

            bgra_premul.push(b_p);
            bgra_premul.push(g_p);
            bgra_premul.push(r_p);
            bgra_premul.push(alpha);
        }

        if let Some(image_pixmap) = tiny_skia::PixmapRef::from_bytes(&bgra_premul, width, height) {
            let paint = tiny_skia::PixmapPaint {
                quality: tiny_skia::FilterQuality::Bilinear,
                ..tiny_skia::PixmapPaint::default()
            };

            let physical_w = logical_width.apply_scale(self.scale).value();
            let physical_h = logical_height.apply_scale(self.scale).value();

            let px_w = f32::from(u16::try_from(width).unwrap_or(1));
            let px_h = f32::from(u16::try_from(height).unwrap_or(1));
            let scale_x = physical_w / px_w;
            let scale_y = physical_h / px_h;

            let physical_x = x.apply_scale(self.scale).value();
            let physical_y = y.apply_scale(self.scale).value();

            let transform =
                Transform::from_scale(scale_x, scale_y).post_translate(physical_x, physical_y);

            pixmap.draw_pixmap(0, 0, image_pixmap, &paint, transform, None);
        }
    }
}

/// Scales every pixel of `glyph_pixmap` (filled with the text's paint) by the coverage `mask`
/// cosmic-text rasterized for the glyph, leaving a premultiplied glyph ready to composite.
fn apply_glyph_mask(glyph_pixmap: &mut tiny_skia::Pixmap, mask: &[u8]) {
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
