use super::canvas::TinySkiaCosmicCanvas;
use super::glyph::PlacedGlyph;
use super::shaping::ShapingFont;
use crate::shared::primitives::geometry::{LogicalPx, Rect as LogicalRect, Scale, Size};
use crate::shared::rendering::ports::draw::RgbaImage;
use crate::shared::rendering::ports::text::{TextPlacement, TextRun};
use cosmic_text::{Buffer, SwashContent};
use tiny_skia::Transform;

impl TinySkiaCosmicCanvas<'_> {
    pub(crate) fn draw_text_impl(&mut self, run: &TextRun<'_>, placement: TextPlacement) {
        let Some(pixmap) = &mut self.pixmap else {
            return;
        };
        let font = ShapingFont::resolve(run.font(), &self.default_font, self.scale);
        let buffer = font.shape(&mut self.fonts.font_system, run.text());
        let physical_x = aligned_x(placement, &buffer, self.scale);
        let physical_y = scaled_coordinate(placement.origin().y(), self.scale);

        for line in buffer.layout_runs() {
            for glyph in line.glyphs {
                let physical_glyph = glyph.physical((physical_x, physical_y + line.line_y), 1.0);

                if let Some(image) = self
                    .fonts
                    .swash_cache
                    .get_image(&mut self.fonts.font_system, physical_glyph.cache_key)
                    && image.content == SwashContent::Mask
                {
                    PlacedGlyph::new(image, &physical_glyph).draw(pixmap, run.color());
                }
            }
        }
    }

    pub(crate) fn draw_image_impl(&mut self, image: RgbaImage<'_>, area: LogicalRect) {
        let Some(pixmap) = &mut self.pixmap else {
            return;
        };
        let width = image.size().width();
        let height = image.size().height();
        let bgra_premul = premultiplied_bgra(image.data());

        if let Some(image_pixmap) = tiny_skia::PixmapRef::from_bytes(&bgra_premul, width, height) {
            let paint = tiny_skia::PixmapPaint {
                quality: tiny_skia::FilterQuality::Bilinear,
                ..tiny_skia::PixmapPaint::default()
            };
            let transform = image_transform(area, image.size(), self.scale);
            pixmap.draw_pixmap(0, 0, image_pixmap, &paint, transform, None);
        }
    }
}

/// Where the text in `buffer` starts, in physical pixels: the placement's origin, moved right
/// as its alignment asks within the width it has.
fn aligned_x(placement: TextPlacement, buffer: &Buffer, scale: Scale) -> f32 {
    let alignment = placement.alignment();
    let origin_x = scaled_coordinate(placement.origin().x(), scale);
    let text_width = buffer
        .layout_runs()
        .fold(0.0_f32, |widest, run| widest.max(run.line_w));
    let available = LogicalPx::new(f32::from(
        u16::try_from(alignment.available_width()).unwrap_or(u16::MAX),
    ))
    .apply_scale(scale)
    .value();
    origin_x + alignment.align().offset(available, text_width)
}

/// A logical `coordinate` (0 when it does not fit an `i16`) in physical pixels at `scale`.
fn scaled_coordinate(coordinate: i32, scale: Scale) -> f32 {
    let logical = f32::from(i16::try_from(coordinate).unwrap_or(0));
    LogicalPx::new(logical).apply_scale(scale).value()
}

/// A logical `length` (0 when it does not fit a `u16`) in physical pixels at `scale`.
fn scaled_length(length: u32, scale: Scale) -> f32 {
    let logical = f32::from(u16::try_from(length).unwrap_or(0));
    LogicalPx::new(logical).apply_scale(scale).value()
}

/// The RGBA `pixels` as premultiplied BGRA, the layout tiny-skia composites.
fn premultiplied_bgra(pixels: &[u8]) -> Vec<u8> {
    let mut bgra_premul = Vec::with_capacity(pixels.len());
    for &[red, green, blue, alpha] in pixels.as_chunks::<4>().0 {
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
    bgra_premul
}

/// Stretches an image of `size` pixels over `area`, in physical pixels at `scale`.
fn image_transform(area: LogicalRect, size: Size, scale: Scale) -> Transform {
    let physical_w = scaled_length(area.width(), scale);
    let physical_h = scaled_length(area.height(), scale);

    let px_w = f32::from(u16::try_from(size.width()).unwrap_or(1));
    let px_h = f32::from(u16::try_from(size.height()).unwrap_or(1));
    let scale_x = physical_w / px_w;
    let scale_y = physical_h / px_h;

    let physical_x = scaled_coordinate(area.x(), scale);
    let physical_y = scaled_coordinate(area.y(), scale);

    Transform::from_scale(scale_x, scale_y).post_translate(physical_x, physical_y)
}
