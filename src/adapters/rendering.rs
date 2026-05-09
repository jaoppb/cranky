use crate::ports::canvas::{Canvas};
use crate::domain::color::{DrawingColor, Color as DomainColor};
use crate::domain::errors::PortError;
use tiny_skia::{
    Color as SkiaColor, Paint, PixmapMut, Rect, Transform, PathBuilder, FillRule, 
    GradientStop, LinearGradient, Point, SpreadMode, Stroke, LineCap, LineJoin, Mask
};
use cosmic_text::{
    Attrs, Buffer, Family, FontSystem, Metrics, Shaping, SwashCache, SwashContent,
};

pub struct TinySkiaCosmicCanvas<'a> {
    pixmap: PixmapMut<'a>,
    font_system: &'a mut FontSystem,
    swash_cache: &'a mut SwashCache,
    scale: f32,
}

impl<'a> TinySkiaCosmicCanvas<'a> {
    pub fn new(
        pixmap: PixmapMut<'a>,
        font_system: &'a mut FontSystem,
        swash_cache: &'a mut SwashCache,
        scale: f32,
    ) -> Self {
        Self {
            pixmap,
            font_system,
            swash_cache,
            scale,
        }
    }

    fn to_skia_color(color: DomainColor) -> SkiaColor {
        SkiaColor::from_rgba8(color.r(), color.g(), color.b(), color.a())
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
    fn draw_rect(&mut self, x: f32, y: f32, width: f32, height: f32, color: DrawingColor, radius: f32) {
        if let Some(physical_rect) = Rect::from_xywh(x * self.scale, y * self.scale, width * self.scale, height * self.scale) {
            let paint = self.get_paint(color, physical_rect);
            let r = (radius * self.scale).min(physical_rect.width() / 2.0).min(physical_rect.height() / 2.0);

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

    fn draw_border(&mut self, x: f32, y: f32, width: f32, height: f32, color: DrawingColor, radius: f32, size: f32) {
        if let Some(physical_rect) = Rect::from_xywh(x * self.scale, y * self.scale, width * self.scale, height * self.scale) {
            let paint = self.get_paint(color, physical_rect);
            let stroke = Stroke {
                width: size * self.scale,
                miter_limit: 4.0,
                line_cap: LineCap::Butt,
                line_join: LineJoin::Miter,
                dash: None,
            };

            let r = (radius * self.scale).min(physical_rect.width() / 2.0).min(physical_rect.height() / 2.0);
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

    fn measure_text(&mut self, text: &str, font_family: &str, font_size: f32) -> (f32, f32) {
        let metrics = Metrics::new(font_size * self.scale, font_size * 1.4 * self.scale);
        let mut buffer = Buffer::new(self.font_system, metrics);
        let attrs = Attrs::new().family(Self::get_family(font_family));

        buffer.set_text(self.font_system, text, &attrs, Shaping::Advanced, None);
        buffer.shape_until_scroll(self.font_system, false);

        let mut width: f32 = 0.0;
        let mut height: f32 = 0.0;
        for run in buffer.layout_runs() {
            width = width.max(run.line_w);
            height += metrics.line_height;
        }

        (width / self.scale, height / self.scale)
    }

    fn draw_text(&mut self, text: &str, font_family: &str, font_size: f32, color: DrawingColor, x: f32, y: f32) {
        let metrics = Metrics::new(font_size * self.scale, font_size * 1.4 * self.scale);
        let mut buffer = Buffer::new(self.font_system, metrics);
        let attrs = Attrs::new().family(Self::get_family(font_family));

        buffer.set_text(self.font_system, text, &attrs, Shaping::Advanced, None);
        buffer.shape_until_scroll(self.font_system, false);

        for run in buffer.layout_runs() {
            for glyph in run.glyphs {
                let physical_glyph = glyph.physical(
                    (x * self.scale, y * self.scale + run.line_y),
                    1.0,
                );
                
                if let Some(image) = self.swash_cache.get_image(self.font_system, physical_glyph.cache_key) {
                    if let SwashContent::Mask = image.content {
                        if let Some(physical_rect) = Rect::from_xywh(
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

                            let mut mask = Mask::new(image.placement.width, image.placement.height).unwrap();
                            mask.data_mut().copy_from_slice(&image.data);
                            
                            self.pixmap.fill_rect(physical_rect, &paint, Transform::identity(), Some(&mask));
                        }
                    }
                }
            }
        }
    }

    fn flush(&mut self) -> Result<(), PortError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tiny_skia::Pixmap;
    use crate::domain::color::Color;

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
                1.0
            );

            canvas.draw_rect(
                10.0, 10.0, 80.0, 80.0, 
                DrawingColor::Solid(Color::new(255, 0, 0, 255)), 
                0.0
            );
        }

        // Check a pixel inside the rect
        let pixel = pixmap.pixel(50, 50).unwrap();
        assert_eq!(pixel.red(), 255);
        assert_eq!(pixel.green(), 0);
        assert_eq!(pixel.blue(), 0);
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
            1.0
        );

        let (w, h) = canvas.measure_text("test", "", 14.0);
        assert!(w > 0.0);
        assert!(h > 0.0);
    }
}
