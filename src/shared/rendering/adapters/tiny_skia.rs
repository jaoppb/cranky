use crate::shared::primitives::color::{Color as DomainColor, DrawingColor};
use crate::shared::rendering::ports::canvas::Canvas;

use cosmic_text::{Attrs, Buffer, Family, FontSystem, Metrics, Shaping, SwashCache, SwashContent};
use tiny_skia::{
    Color as SkiaColor, FillRule, GradientStop, LineCap, LineJoin, LinearGradient, Paint,
    PathBuilder, PixmapMut, Point, Rect, SpreadMode, Stroke, Transform,
};

use crate::shared::config::domain::{FontFamily, FontSize};
use crate::shared::primitives::geometry::{LogicalPx, PhysicalPx, Position, Scale, Size};

pub struct TinySkiaCanvasFactory {
    font_system: FontSystem,
    swash_cache: SwashCache,
}

impl Default for TinySkiaCanvasFactory {
    fn default() -> Self {
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

impl crate::shared::rendering::ports::canvas::CanvasFactory for TinySkiaCanvasFactory {
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

pub struct TinySkiaCosmicCanvas<'a> {
    pixmap: Option<PixmapMut<'a>>,
    font_system: &'a mut FontSystem,
    swash_cache: &'a mut SwashCache,
    scale: Scale,
    default_font_family: FontFamily,
    default_font_size: FontSize,
}

impl<'a> TinySkiaCosmicCanvas<'a> {
    #[must_use]
    pub const fn new(
        pixmap: PixmapMut<'a>,
        font_system: &'a mut FontSystem,
        swash_cache: &'a mut SwashCache,
        scale: Scale,
        default_font_family: FontFamily,
        default_font_size: FontSize,
    ) -> Self {
        Self {
            pixmap: Some(pixmap),
            font_system,
            swash_cache,
            scale,
            default_font_family,
            default_font_size,
        }
    }

    #[must_use]
    pub const fn from_optional_pixmap(
        pixmap: Option<PixmapMut<'a>>,
        font_system: &'a mut FontSystem,
        swash_cache: &'a mut SwashCache,
        scale: Scale,
        default_font_family: FontFamily,
        default_font_size: FontSize,
    ) -> Self {
        Self {
            pixmap,
            font_system,
            swash_cache,
            scale,
            default_font_family,
            default_font_size,
        }
    }

    fn to_skia_color(color: DomainColor) -> SkiaColor {
        SkiaColor::from_rgba8(color.b(), color.g(), color.r(), color.a())
    }

    fn get_paint(color: &DrawingColor, rect: Rect) -> Paint<'static> {
        let mut paint = Paint {
            anti_alias: true,
            ..Paint::default()
        };

        match color {
            DrawingColor::Solid(c) => {
                paint.set_color(Self::to_skia_color(*c));
            }
            DrawingColor::Gradient(colors, angle) => {
                let count = colors.len().saturating_sub(1).max(1);
                let stops: Vec<GradientStop> = colors
                    .iter()
                    .enumerate()
                    .map(|(i, &c)| {
                        #[allow(clippy::as_conversions, clippy::cast_precision_loss)]
                        let pos = (i as f32) / (count as f32);
                        GradientStop::new(pos, Self::to_skia_color(c))
                    })
                    .collect();

                let angle_rad = angle.to_radians();
                let center_x = rect.left() + rect.width() / 2.0;
                let center_y = rect.top() + rect.height() / 2.0;

                let distance = (rect.width() / 2.0 * angle_rad.cos()).abs()
                    + (rect.height() / 2.0 * angle_rad.sin()).abs();

                let x_offset = angle_rad.cos() * distance;
                let y_offset = angle_rad.sin() * distance;

                let start = Point::from_xy(center_x - x_offset, center_y - y_offset);
                let end = Point::from_xy(center_x + x_offset, center_y + y_offset);

                if let Some(shader) =
                    LinearGradient::new(start, end, stops, SpreadMode::Pad, Transform::identity())
                {
                    paint.shader = shader;
                } else if let Some(&c) = colors.first() {
                    paint.set_color(Self::to_skia_color(c));
                }
            }
        }
        paint
    }

    #[must_use]
    pub const fn get_family(name: &str) -> Family<'_> {
        if name.eq_ignore_ascii_case("monospace") {
            Family::Monospace
        } else if name.eq_ignore_ascii_case("serif") {
            Family::Serif
        } else if name.eq_ignore_ascii_case("sans-serif") {
            Family::SansSerif
        } else if name.eq_ignore_ascii_case("cursive") {
            Family::Cursive
        } else if name.eq_ignore_ascii_case("fantasy") {
            Family::Fantasy
        } else if name.is_empty() {
            Family::Monospace
        } else {
            Family::Name(name)
        }
    }
}

impl Canvas for TinySkiaCosmicCanvas<'_> {
    #[allow(clippy::many_single_char_names)]
    fn draw_rect(
        &mut self,
        x: LogicalPx,
        y: LogicalPx,
        width: LogicalPx,
        height: LogicalPx,
        color: DrawingColor,
        radius: LogicalPx,
    ) {
        let Some(pixmap) = &mut self.pixmap else {
            return;
        };
        let physical_x = x.apply_scale(&self.scale).value();
        let physical_y = y.apply_scale(&self.scale).value();
        let physical_w = width.apply_scale(&self.scale).value();
        let physical_h = height.apply_scale(&self.scale).value();

        if let Some(physical_rect) = Rect::from_xywh(physical_x, physical_y, physical_w, physical_h)
        {
            let paint = Self::get_paint(&color, physical_rect);
            let r = radius
                .apply_scale(&self.scale)
                .value()
                .min(physical_rect.width() / 2.0)
                .min(physical_rect.height() / 2.0);

            if r <= 0.0 {
                pixmap.fill_rect(physical_rect, &paint, Transform::identity(), None);
            } else {
                let mut pb = PathBuilder::new();
                let (x, y, w, h) = (
                    physical_rect.left(),
                    physical_rect.top(),
                    physical_rect.width(),
                    physical_rect.height(),
                );
                pb.move_to(x + r, y);
                pb.line_to(x + w - r, y);
                pb.quad_to(x + w, y, x + w, y + r);
                pb.line_to(x + w, y + h - r);
                pb.quad_to(x + w, y + h, x + w - r, y + h);
                pb.line_to(x + r, y + h);
                pb.quad_to(x, y + h, x, y + h - r);
                pb.line_to(x, y + r);
                pb.quad_to(x, y, x + r, y);
                pb.close();

                if let Some(path) = pb.finish() {
                    pixmap.fill_path(
                        &path,
                        &paint,
                        FillRule::Winding,
                        Transform::identity(),
                        None,
                    );
                }
            }
        }
    }

    #[allow(clippy::as_conversions, clippy::cast_precision_loss, clippy::many_single_char_names)]
    fn draw_border(
        &mut self,
        position: Position,
        size: Size,
        color: DrawingColor,
        radius: LogicalPx,
        border_size: LogicalPx,
    ) {
        let Some(pixmap) = &mut self.pixmap else {
            return;
        };
        let x = LogicalPx::new(position.x() as f32);
        let y = LogicalPx::new(position.y() as f32);
        let width = LogicalPx::new(size.width() as f32);
        let height = LogicalPx::new(size.height() as f32);
        let physical_x = x.apply_scale(&self.scale).value();
        let physical_y = y.apply_scale(&self.scale).value();
        let physical_w = width.apply_scale(&self.scale).value();
        let physical_h = height.apply_scale(&self.scale).value();
        let stroke_w = border_size.apply_scale(&self.scale).value();

        if stroke_w <= 0.0 {
            return;
        }

        if let Some(physical_rect) = Rect::from_xywh(physical_x, physical_y, physical_w, physical_h)
        {
            let paint = Self::get_paint(&color, physical_rect);
            let stroke = Stroke {
                width: stroke_w,
                miter_limit: 4.0,
                line_cap: LineCap::Butt,
                line_join: LineJoin::Miter,
                dash: None,
            };

            let half_stroke = stroke_w / 2.0;
            let (x, y, w, h) = (
                physical_rect.left() + half_stroke,
                physical_rect.top() + half_stroke,
                (physical_rect.width() - stroke_w).max(0.0),
                (physical_rect.height() - stroke_w).max(0.0),
            );

            let max_r = (w / 2.0).min(h / 2.0);
            let r = (radius.apply_scale(&self.scale).value() - half_stroke).clamp(0.0, max_r);

            let mut pb = PathBuilder::new();
            if r <= 0.0 {
                pb.move_to(x, y);
                pb.line_to(x + w, y);
                pb.line_to(x + w, y + h);
                pb.line_to(x, y + h);
            } else {
                pb.move_to(x + r, y);
                pb.line_to(x + w - r, y);
                pb.quad_to(x + w, y, x + w, y + r);
                pb.line_to(x + w, y + h - r);
                pb.quad_to(x + w, y + h, x + w - r, y + h);
                pb.line_to(x + r, y + h);
                pb.quad_to(x, y + h, x, y + h - r);
                pb.line_to(x, y + r);
                pb.quad_to(x, y, x + r, y);
            }
            pb.close();

            if let Some(path) = pb.finish() {
                pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
            }
        }
    }

    #[allow(clippy::as_conversions, clippy::cast_precision_loss)]
    fn draw_text(
        &mut self,
        text: &str,
        font_family: Option<&FontFamily>,
        font_size: Option<FontSize>,
        color: DrawingColor,
        position: Position,
    ) {
        let Some(pixmap) = &mut self.pixmap else {
            return;
        };
        let size = font_size.unwrap_or(self.default_font_size).value();
        let family = font_family.unwrap_or(&self.default_font_family).as_str();
        let physical_x = LogicalPx::new(position.x() as f32)
            .apply_scale(&self.scale)
            .value();
        let physical_y = LogicalPx::new(position.y() as f32)
            .apply_scale(&self.scale)
            .value();

        let physical_size = LogicalPx::new(size).apply_scale(&self.scale).value();
        let metrics = Metrics::new(physical_size, physical_size * 1.0);
        let mut buffer = Buffer::new(self.font_system, metrics);
        let attrs = Attrs::new().family(Self::get_family(family));

        buffer.set_text(text, &attrs, Shaping::Advanced, None);
        buffer.shape_until_scroll(self.font_system, false);

        for run in buffer.layout_runs() {
            for glyph in run.glyphs {
                let physical_glyph = glyph.physical((physical_x, physical_y + run.line_y), 1.0);

                if let Some(image) = self
                    .swash_cache
                    .get_image(self.font_system, physical_glyph.cache_key)
                    && image.content == SwashContent::Mask
                    && let Some(physical_rect) = Rect::from_xywh(
                        (physical_glyph.x.saturating_add(image.placement.left)) as f32,
                        (physical_glyph.y.saturating_sub(image.placement.top)) as f32,
                        image.placement.width as f32,
                        image.placement.height as f32,
                    )
                {
                    let paint = Self::get_paint(&color, physical_rect);

                    if image.placement.width > 0
                        && image.placement.height > 0
                        && let Some(mut glyph_pixmap) =
                            tiny_skia::Pixmap::new(image.placement.width, image.placement.height)
                        && let Some(glyph_rect) = Rect::from_xywh(
                            0.0,
                            0.0,
                            image.placement.width as f32,
                            image.placement.height as f32,
                        )
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

    #[allow(clippy::as_conversions, clippy::cast_precision_loss)]
    fn draw_image(
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
        let logical_width = LogicalPx::new(logical_size.width() as f32);
        let logical_height = LogicalPx::new(logical_size.height() as f32);
        let x = LogicalPx::new(position.x() as f32);
        let y = LogicalPx::new(position.y() as f32);
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

            let scale_x = physical_w / (width as f32);
            let scale_y = physical_h / (height as f32);

            let physical_x = x.apply_scale(&self.scale).value();
            let physical_y = y.apply_scale(&self.scale).value();

            let transform =
                Transform::from_scale(scale_x, scale_y).post_translate(physical_x, physical_y);

            pixmap.draw_pixmap(0, 0, image_pixmap, &paint, transform, None);
        }
    }
}

pub struct CosmicTextMeasurer<'a> {
    font_system: &'a mut FontSystem,
    scale: Scale,
    default_font_family: FontFamily,
    default_font_size: FontSize,
}

impl<'a> CosmicTextMeasurer<'a> {
    #[must_use]
    pub const fn new(
        font_system: &'a mut FontSystem,
        scale: Scale,
        default_font_family: FontFamily,
        default_font_size: FontSize,
    ) -> Self {
        Self {
            font_system,
            scale,
            default_font_family,
            default_font_size,
        }
    }
}

impl crate::features::layout_engine::domain::TextMeasurer for CosmicTextMeasurer<'_> {
    #[allow(clippy::as_conversions, clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    fn measure(
        &mut self,
        text: &str,
        font_family: Option<&FontFamily>,
        font_size: Option<FontSize>,
    ) -> Size {
        let size = font_size.unwrap_or(self.default_font_size).value();
        let family = font_family.unwrap_or(&self.default_font_family).as_str();

        let physical_size = LogicalPx::new(size).apply_scale(&self.scale).value();
        let metrics = Metrics::new(physical_size, physical_size * 1.0);
        let mut buffer = Buffer::new(self.font_system, metrics);
        let attrs = Attrs::new().family(TinySkiaCosmicCanvas::get_family(family));

        buffer.set_text(text, &attrs, Shaping::Advanced, None);
        buffer.shape_until_scroll(self.font_system, false);

        let mut physical_width: f32 = 0.0;
        let mut physical_height: f32 = 0.0;
        for run in buffer.layout_runs() {
            physical_width = physical_width.max(run.line_w);
            physical_height += metrics.line_height;
        }

        let w = PhysicalPx::new(physical_width).apply_inverse_scale(&self.scale);
        let h = PhysicalPx::new(physical_height).apply_inverse_scale(&self.scale);

        Size::new(
            w.value().ceil().max(0.0) as u32,
            h.value().ceil().max(0.0) as u32,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::layout_engine::domain::TextMeasurer;
    use crate::shared::primitives::color::Color;
    use tiny_skia::Pixmap;

    #[test]
    fn test_canvas_draw_rect() {
        let mut pixmap = Pixmap::new(100, 100).unwrap();
        let mut font_system = FontSystem::new();
        let mut swash_cache = SwashCache::new();

        {
            let mut canvas = TinySkiaCosmicCanvas::new(
                pixmap.as_mut(),
                &mut font_system,
                &mut swash_cache,
                Scale::new(1.0),
                FontFamily::new("sans-serif".to_string()),
                FontSize::new(14.0),
            );

            canvas.draw_rect(
                LogicalPx::new(10.0),
                LogicalPx::new(10.0),
                LogicalPx::new(80.0),
                LogicalPx::new(80.0),
                DrawingColor::Solid(Color::new(255, 0, 0, 255)),
                LogicalPx::new(0.0),
            );
        }

        // Check a pixel inside the rect
        let pixel = pixmap.pixel(50, 50).unwrap();
        assert_eq!(pixel.red(), 0);
        assert_eq!(pixel.green(), 0);
        assert_eq!(pixel.blue(), 255);
        assert_eq!(pixel.alpha(), 255);
    }

    #[test]
    fn test_canvas_measure_text() {
        let mut font_system = FontSystem::new();

        let mut measurer = CosmicTextMeasurer::new(
            &mut font_system,
            Scale::new(1.0),
            FontFamily::new("sans-serif".to_string()),
            FontSize::new(14.0),
        );

        let size = measurer.measure("test", None, None);
        assert!(size.width() > 0);
        assert!(size.height() > 0);
    }

    #[test]
    fn test_canvas_draw_text() {
        let mut pixmap = Pixmap::new(100, 100).unwrap();
        let mut font_system = FontSystem::new();
        let mut swash_cache = SwashCache::new();

        let mut canvas = TinySkiaCosmicCanvas::new(
            pixmap.as_mut(),
            &mut font_system,
            &mut swash_cache,
            Scale::new(1.0),
            FontFamily::new("sans-serif".to_string()),
            FontSize::new(14.0),
        );

        // This should not panic
        canvas.draw_text(
            "test ",
            None,
            None,
            DrawingColor::Solid(Color::new(255, 255, 255, 255)),
            Position::new(10, 10),
        );

        // Verify that at least some pixels were drawn (text is white)
        let mut drawn = false;
        for pixel in pixmap.pixels() {
            if pixel.alpha() > 0 {
                drawn = true;
                break;
            }
        }
        assert!(drawn, "Text should have drawn some pixels");
    }

    #[test]
    fn test_canvas_factory() {
        use crate::shared::rendering::ports::canvas::CanvasFactory;
        let mut factory = TinySkiaCanvasFactory::new();
        let mut data = vec![0; 100 * 100 * 4];
        {
            let _canvas = factory.create_canvas(
                &mut data,
                Size::new(100, 100),
                Scale::new(1.0),
                FontFamily::new("sans-serif".to_string()),
                FontSize::new(14.0),
            );
        }
        let _measurer = factory.create_text_measurer(
            Scale::new(1.0),
            FontFamily::new("sans-serif".to_string()),
            FontSize::new(14.0),
        );
    }

    #[test]
    fn test_canvas_draw_border() {
        let mut pixmap = Pixmap::new(100, 100).unwrap();
        let mut font_system = FontSystem::new();
        let mut swash_cache = SwashCache::new();

        {
            let mut canvas = TinySkiaCosmicCanvas::new(
                pixmap.as_mut(),
                &mut font_system,
                &mut swash_cache,
                Scale::new(1.0),
                FontFamily::new("sans-serif".to_string()),
                FontSize::new(14.0),
            );

            canvas.draw_border(
                Position::new(0, 0),
                Size::new(20, 20),
                DrawingColor::Solid(Color::new(0, 255, 0, 255)),
                LogicalPx::new(0.0), // No radius
                LogicalPx::new(2.0),
            );

            canvas.draw_border(
                Position::new(20, 20),
                Size::new(60, 60),
                DrawingColor::Solid(Color::new(0, 0, 255, 255)),
                LogicalPx::new(10.0), // With radius
                LogicalPx::new(2.0),
            );
        }

        // Top-left outer corner must be filled (green)
        let p0 = pixmap.pixel(0, 0).unwrap();
        assert_eq!(p0.green(), 255);
        assert_eq!(p0.alpha(), 255);

        // Interior should be transparent
        let p_inside = pixmap.pixel(10, 10).unwrap();
        assert_eq!(p_inside.alpha(), 0);
    }

    #[test]
    fn test_canvas_draw_rect_with_radius() {
        let mut pixmap = Pixmap::new(100, 100).unwrap();
        let mut font_system = FontSystem::new();
        let mut swash_cache = SwashCache::new();

        let mut canvas = TinySkiaCosmicCanvas::new(
            pixmap.as_mut(),
            &mut font_system,
            &mut swash_cache,
            Scale::new(1.0),
            FontFamily::new("sans-serif".to_string()),
            FontSize::new(14.0),
        );

        canvas.draw_rect(
            LogicalPx::new(10.0),
            LogicalPx::new(10.0),
            LogicalPx::new(80.0),
            LogicalPx::new(80.0),
            DrawingColor::Gradient(
                vec![Color::new(255, 0, 0, 255), Color::new(0, 255, 0, 255)],
                45.0,
            ),
            LogicalPx::new(20.0), // With radius
        );
    }

    #[test]
    fn test_canvas_draw_text_gradient() {
        let mut pixmap = Pixmap::new(100, 100).unwrap();
        let mut font_system = FontSystem::new();
        let mut swash_cache = SwashCache::new();

        let mut canvas = TinySkiaCosmicCanvas::new(
            pixmap.as_mut(),
            &mut font_system,
            &mut swash_cache,
            Scale::new(1.0),
            FontFamily::new("sans-serif".to_string()),
            FontSize::new(14.0),
        );

        canvas.draw_text(
            "gradient text",
            None,
            None,
            DrawingColor::Gradient(
                vec![Color::new(255, 0, 0, 255), Color::new(0, 255, 0, 255)],
                0.0,
            ),
            Position::new(10, 10),
        );
    }

    #[test]
    fn test_canvas_draw_image() {
        let mut pixmap = Pixmap::new(100, 100).unwrap();
        let mut font_system = FontSystem::new();
        let mut swash_cache = SwashCache::new();

        let mut canvas = TinySkiaCosmicCanvas::new(
            pixmap.as_mut(),
            &mut font_system,
            &mut swash_cache,
            Scale::new(1.0),
            FontFamily::new("sans-serif".to_string()),
            FontSize::new(14.0),
        );

        let image_data = vec![
            255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 0, 255,
        ];

        canvas.draw_image(
            &image_data,
            Size::new(2, 2),
            Size::new(20, 20),
            Position::new(10, 10),
        );
    }
}
