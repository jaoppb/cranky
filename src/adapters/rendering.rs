use crate::ports::canvas::{Canvas};
use crate::domain::shared::color::{DrawingColor, Color as DomainColor};
use crate::ports::DisplayServerError;
use tiny_skia::{
    Color as SkiaColor, Paint, PixmapMut, Rect, Transform, PathBuilder, FillRule, 
    GradientStop, LinearGradient, Point, SpreadMode, Stroke, LineCap, LineJoin
};
use cosmic_text::{
    Attrs, Buffer, Family, FontSystem, Metrics, Shaping, SwashCache, SwashContent,
};

use crate::domain::config::{FontFamily, FontSize};
use crate::domain::shared::geometry::{Position, Scale, LogicalPx};

pub struct TinySkiaCosmicCanvas<'a> {
    pixmap: PixmapMut<'a>,
    font_system: &'a mut FontSystem,
    swash_cache: &'a mut SwashCache,
    scale: Scale,
    default_font_family: FontFamily,
    default_font_size: FontSize,
}

impl<'a> TinySkiaCosmicCanvas<'a> {
    pub fn new(
        pixmap: PixmapMut<'a>,
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

    fn get_paint(&self, color: DrawingColor, rect: Rect) -> Paint<'static> {
        let mut paint = Paint {
            anti_alias: true,
            ..Paint::default()
        };

        match color {
            DrawingColor::Solid(c) => {
                paint.set_color(Self::to_skia_color(c));
            }
            DrawingColor::Gradient(colors, angle) => {
                let stops: Vec<GradientStop> = colors
                    .iter()
                    .enumerate()
                    .map(|(i, &c)| {
                        GradientStop::new(
                            i as f32 / (colors.len() - 1).max(1) as f32,
                            Self::to_skia_color(c)
                        )
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

    fn get_family(name: &str) -> Family<'_> {
        match name.to_lowercase().as_str() {
            "monospace" => Family::Monospace,
            "serif" => Family::Serif,
            "sans-serif" => Family::SansSerif,
            "cursive" => Family::Cursive,
            "fantasy" => Family::Fantasy,
            _ => if name.is_empty() { Family::Monospace } else { Family::Name(name) },
        }
    }
}

impl<'a> Canvas for TinySkiaCosmicCanvas<'a> {
    fn clear(&mut self) {
        self.pixmap.fill(tiny_skia::Color::TRANSPARENT);
    }

    fn draw_rect(&mut self, x: LogicalPx, y: LogicalPx, width: LogicalPx, height: LogicalPx, color: DrawingColor, radius: LogicalPx) {
        let physical_x = x.apply_scale(&self.scale).value();
        let physical_y = y.apply_scale(&self.scale).value();
        let physical_w = width.apply_scale(&self.scale).value();
        let physical_h = height.apply_scale(&self.scale).value();
        
        if let Some(physical_rect) = Rect::from_xywh(physical_x, physical_y, physical_w, physical_h) {
            let paint = self.get_paint(color, physical_rect);
            let r = radius.apply_scale(&self.scale).value().min(physical_rect.width() / 2.0).min(physical_rect.height() / 2.0);

            if r <= 0.0 {
                self.pixmap.fill_rect(physical_rect, &paint, Transform::identity(), None);
            } else {
                let mut pb = PathBuilder::new();
                let (x, y, w, h) = (physical_rect.left(), physical_rect.top(), physical_rect.width(), physical_rect.height());
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
                    self.pixmap.fill_path(&path, &paint, FillRule::Winding, Transform::identity(), None);
                }
            }
        }
    }

    fn draw_border(&mut self, x: LogicalPx, y: LogicalPx, width: LogicalPx, height: LogicalPx, color: DrawingColor, radius: LogicalPx, size: LogicalPx) {
        let physical_x = x.apply_scale(&self.scale).value();
        let physical_y = y.apply_scale(&self.scale).value();
        let physical_w = width.apply_scale(&self.scale).value();
        let physical_h = height.apply_scale(&self.scale).value();
        
        if let Some(physical_rect) = Rect::from_xywh(physical_x, physical_y, physical_w, physical_h) {
            let paint = self.get_paint(color, physical_rect);
            let stroke = Stroke {
                width: size.apply_scale(&self.scale).value(),
                miter_limit: 4.0,
                line_cap: LineCap::Butt,
                line_join: LineJoin::Miter,
                dash: None,
            };

            let r = radius.apply_scale(&self.scale).value().min(physical_rect.width() / 2.0).min(physical_rect.height() / 2.0);
            let mut pb = PathBuilder::new();
            let (x, y, w, h) = (physical_rect.left(), physical_rect.top(), physical_rect.width(), physical_rect.height());
            
            if r <= 0.0 {
                pb.move_to(x, y);
                pb.line_to(x + w, y);
                pb.line_to(x + w, y + h);
                pb.line_to(x, y + h);
                pb.close();
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
                pb.close();
            }

            if let Some(path) = pb.finish() {
                self.pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
            }
        }
    }

    fn measure_text(&mut self, text: &str, font_family: Option<&FontFamily>, font_size: Option<FontSize>) -> (LogicalPx, LogicalPx) {
        let size = font_size.unwrap_or(self.default_font_size).value();
        let family = font_family.unwrap_or(&self.default_font_family).as_str();
        
        let physical_size = LogicalPx::new(size).apply_scale(&self.scale).value();
        let metrics = Metrics::new(physical_size, physical_size * 1.0);
        let mut buffer = Buffer::new(self.font_system, metrics);
        let attrs = Attrs::new().family(Self::get_family(family));

        buffer.set_text(text, &attrs, Shaping::Advanced, None);
        buffer.shape_until_scroll(self.font_system, false);

        let mut physical_width: f32 = 0.0;
        let mut physical_height: f32 = 0.0;
        for run in buffer.layout_runs() {
            physical_width = physical_width.max(run.line_w);
            physical_height += metrics.line_height;
        }

        use crate::domain::shared::geometry::PhysicalPx;
        (PhysicalPx::new(physical_width).apply_inverse_scale(&self.scale), PhysicalPx::new(physical_height).apply_inverse_scale(&self.scale))
    }

    fn draw_text(&mut self, text: &str, font_family: Option<&FontFamily>, font_size: Option<FontSize>, color: DrawingColor, position: Position) {
        let size = font_size.unwrap_or(self.default_font_size).value();
        let family = font_family.unwrap_or(&self.default_font_family).as_str();
        let physical_x = LogicalPx::new(position.x() as f32).apply_scale(&self.scale).value();
        let physical_y = LogicalPx::new(position.y() as f32).apply_scale(&self.scale).value();
        
        let physical_size = LogicalPx::new(size).apply_scale(&self.scale).value();
        let metrics = Metrics::new(physical_size, physical_size * 1.0);
        let mut buffer = Buffer::new(self.font_system, metrics);
        let attrs = Attrs::new().family(Self::get_family(family));

        buffer.set_text(text, &attrs, Shaping::Advanced, None);
        buffer.shape_until_scroll(self.font_system, false);

        for run in buffer.layout_runs() {
            for glyph in run.glyphs {
                let physical_glyph = glyph.physical(
                    (physical_x, physical_y + run.line_y),
                    1.0,
                );
                
                if let Some(image) = self.swash_cache.get_image(self.font_system, physical_glyph.cache_key)
                    && let SwashContent::Mask = image.content
                        && let Some(physical_rect) = Rect::from_xywh(
                            (physical_glyph.x + image.placement.left) as f32,
                            (physical_glyph.y - image.placement.top) as f32,
                            image.placement.width as f32,
                            image.placement.height as f32
                        ) {
                            let mut paint = Paint {
                                anti_alias: true,
                                ..Paint::default()
                            };

                            match color.clone() {
                                DrawingColor::Solid(c) => {
                                    paint.set_color(Self::to_skia_color(c));
                                }
                                DrawingColor::Gradient(colors, angle) => {
                                    let stops: Vec<GradientStop> = colors
                                        .iter()
                                        .enumerate()
                                        .map(|(i, &c)| {
                                            GradientStop::new(
                                                i as f32 / (colors.len() - 1).max(1) as f32,
                                                Self::to_skia_color(c)
                                            )
                                        })
                                        .collect();

                                    let angle_rad = angle.to_radians();
                                    let center_x = physical_rect.left() + physical_rect.width() / 2.0;
                                    let center_y = physical_rect.top() + physical_rect.height() / 2.0;

                                    let distance = (physical_rect.width() / 2.0 * angle_rad.cos()).abs()
                                        + (physical_rect.height() / 2.0 * angle_rad.sin()).abs();

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

                            if image.placement.width > 0 && image.placement.height > 0
                                && let Some(mut glyph_pixmap) = tiny_skia::Pixmap::new(image.placement.width, image.placement.height)
                                    && let Some(glyph_rect) = Rect::from_xywh(0.0, 0.0, image.placement.width as f32, image.placement.height as f32) {
                                        glyph_pixmap.fill_rect(glyph_rect, &paint, Transform::identity(), None);
                                        
                                        for (pixel, &mask_alpha) in glyph_pixmap.pixels_mut().iter_mut().zip(image.data.iter()) {
                                            let a = (pixel.alpha() as u32 * mask_alpha as u32) / 255;
                                            let r = (pixel.red() as u32 * mask_alpha as u32) / 255;
                                            let g = (pixel.green() as u32 * mask_alpha as u32) / 255;
                                            let b = (pixel.blue() as u32 * mask_alpha as u32) / 255;
                                            if let Some(c) = tiny_skia::PremultipliedColorU8::from_rgba(r as u8, g as u8, b as u8, a as u8) {
                                                *pixel = c;
                                            } else {
                                                *pixel = tiny_skia::PremultipliedColorU8::TRANSPARENT;
                                            }
                                        }
                                        
                                        self.pixmap.draw_pixmap(
                                            physical_glyph.x + image.placement.left,
                                            physical_glyph.y - image.placement.top,
                                            glyph_pixmap.as_ref(),
                                            &tiny_skia::PixmapPaint::default(),
                                            Transform::identity(),
                                            None
                                        );
                                    }
                        }
            }
        }
    }

    fn draw_image(
        &mut self,
        image_data: &[u8],
        width: u32,
        height: u32,
        logical_width: LogicalPx,
        logical_height: LogicalPx,
        x: LogicalPx,
        y: LogicalPx,
    ) {
        
        let mut bgra_premul = Vec::with_capacity(image_data.len());
        for chunk in image_data.chunks_exact(4) {
            let r = chunk[0];
            let g = chunk[1];
            let b = chunk[2];
            let a = chunk[3];
            
            let r_p = (r as u16 * a as u16 / 255) as u8;
            let g_p = (g as u16 * a as u16 / 255) as u8;
            let b_p = (b as u16 * a as u16 / 255) as u8;
            
            bgra_premul.push(b_p);
            bgra_premul.push(g_p);
            bgra_premul.push(r_p);
            bgra_premul.push(a);
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
            
            let transform = Transform::from_scale(scale_x, scale_y)
                .post_translate(physical_x, physical_y);
            
            self.pixmap.draw_pixmap(
                0,
                0,
                image_pixmap,
                &paint,
                transform,
                None
            );
        }
    }

    fn flush(&mut self) -> Result<(), DisplayServerError> {
        Ok(())
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use tiny_skia::Pixmap;
    use crate::domain::shared::color::Color;

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
                FontSize::new(14.0)
            );

            canvas.draw_rect(
                LogicalPx::new(10.0), LogicalPx::new(10.0), LogicalPx::new(80.0), LogicalPx::new(80.0), 
                DrawingColor::Solid(Color::new(255, 0, 0, 255)), 
                LogicalPx::new(0.0)
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
        let mut pixmap = Pixmap::new(100, 100).unwrap();
        let mut font_system = FontSystem::new();
        let mut swash_cache = SwashCache::new();
        
        let mut canvas = TinySkiaCosmicCanvas::new(
            pixmap.as_mut(),
            &mut font_system,
            &mut swash_cache,
            Scale::new(1.0),
            FontFamily::new("sans-serif".to_string()),
            FontSize::new(14.0)
        );

        let (w, h) = canvas.measure_text("test", None, None);
        assert!(w.value() > 0.0);
        assert!(h.value() > 0.0);
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
            FontSize::new(14.0)
        );

        // This should not panic
        canvas.draw_text(
            "test ", 
            None, 
            None, 
            DrawingColor::Solid(Color::new(255, 255, 255, 255)), 
            Position::new(10, 10)
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
}
