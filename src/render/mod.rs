use cosmic_text::{
    Attrs, Buffer, Family, FontSystem, Metrics, Shaping, SwashCache, SwashContent, SwashImage,
};
use log::{debug, info};
use tiny_skia::{Color, Paint, PixmapMut, Rect, Transform};

use crate::config::VerticalAlignment;

#[derive(Clone)]
pub struct TextStyling {
    font_size: f32,
    line_height: f32,
    color: Color,
    font_family: String,
}

impl TextStyling {
    pub fn new(font_size: f32, line_height: f32, color: Color, font_family: String) -> Self {
        Self {
            font_size,
            line_height,
            color,
            font_family,
        }
    }

    pub fn font_size(&self) -> f32 {
        self.font_size
    }

    pub fn line_height(&self) -> f32 {
        self.line_height
    }

    pub fn color(&self) -> Color {
        self.color
    }

    pub fn font_family(&self) -> &str {
        &self.font_family
    }
}

pub struct RenderContext {
    font_system: FontSystem,
    swash_cache: SwashCache,
    vertical_alignment: VerticalAlignment,
    scale: f32,
}

impl RenderContext {
    pub fn new() -> Self {
        Self {
            font_system: FontSystem::new(),
            swash_cache: SwashCache::new(),
            vertical_alignment: VerticalAlignment::default(),
            scale: 1.0,
        }
    }

    pub fn set_vertical_alignment(&mut self, alignment: VerticalAlignment) {
        self.vertical_alignment = alignment;
    }

    pub fn set_scale(&mut self, scale: f32) {
        self.scale = scale;
    }

    pub fn scale(&self) -> f32 {
        self.scale
    }

    pub fn calculate_vertical_offset(&self, area: Rect, content_height: f32) -> f32 {
        match self.vertical_alignment {
            VerticalAlignment::Top => area.top(),
            VerticalAlignment::Center => area.top() + (area.height() - content_height) / 2.0,
            VerticalAlignment::Bottom => area.top() + area.height() - content_height,
        }
    }

    /// Renders text into a pixmap with support for subpixel positioning and various glyph formats.
    pub fn render_text(
        &mut self,
        pixmap: &mut PixmapMut,
        text: &str,
        styling: TextStyling,
        start_x: f32,
        start_y: f32,
    ) {
        info!(
            "Rendering text '{}' at ({}, {}) with scale {}",
            text, start_x, start_y, self.scale
        );
        let metrics = Metrics::new(
            styling.font_size() * self.scale,
            styling.line_height() * self.scale,
        );
        let mut buffer = Buffer::new(&mut self.font_system, metrics);

        let attrs = Attrs::new().family(get_family(styling.font_family()));

        buffer.set_text(&mut self.font_system, text, &attrs, Shaping::Advanced, None);
        buffer.shape_until_scroll(&mut self.font_system, false);

        let mut glyph_count = 0;
        for run in buffer.layout_runs() {
            for glyph in run.glyphs {
                glyph_count += 1;
                // start_x and start_y are in logical units
                // run.line_y is already in physical units (from scaled metrics)
                let physical_glyph = glyph.physical(
                    (start_x * self.scale, start_y * self.scale + run.line_y),
                    1.0,
                );
                let image = match self
                    .swash_cache
                    .get_image(&mut self.font_system, physical_glyph.cache_key)
                {
                    Some(img) => img,
                    None => continue,
                };

                let x = physical_glyph.x + image.placement.left;
                let y = physical_glyph.y - image.placement.top;

                if glyph_count % 10 == 0 || glyph_count < 5 {
                    // Avoid too much log spam
                    debug!(
                        "Glyph at ({}, {}), size {}x{}",
                        x, y, image.placement.width, image.placement.height
                    );
                }

                match image.content {
                    SwashContent::Mask => render_mask_glyph(pixmap, &image, x, y, styling.color()),
                    SwashContent::SubpixelMask => {
                        render_subpixel_glyph(pixmap, &image, x, y, styling.color())
                    }
                    SwashContent::Color => render_color_glyph(pixmap, &image, x, y),
                }
            }
        }
        info!("Rendered {} glyphs for '{}'", glyph_count, text);
    }

    pub fn measure_text(&mut self, text: &str, styling: TextStyling) -> f32 {
        let metrics = Metrics::new(
            styling.font_size() * self.scale,
            styling.line_height() * self.scale,
        );
        let mut buffer = Buffer::new(&mut self.font_system, metrics);

        let attrs = Attrs::new().family(get_family(styling.font_family()));

        buffer.set_text(&mut self.font_system, text, &attrs, Shaping::Advanced, None);
        buffer.shape_until_scroll(&mut self.font_system, false);

        let mut width: f32 = 0.0;
        for run in buffer.layout_runs() {
            width = width.max(run.line_w);
        }
        info!(
            "Measured text '{}': width={}, scaled_width={}",
            text,
            width / self.scale,
            width
        );
        width / self.scale
    }
}

fn get_family(name: &str) -> Family<'_> {
    match name.to_lowercase().as_str() {
        "monospace" => Family::Monospace,
        "serif" => Family::Serif,
        "sans-serif" => Family::SansSerif,
        "cursive" => Family::Cursive,
        "fantasy" => Family::Fantasy,
        "" => Family::Monospace,
        _ => Family::Name(name),
    }
}

fn render_mask_glyph(pixmap: &mut PixmapMut, image: &SwashImage, x: i32, y: i32, color: Color) {
    let mut paint = Paint::default();
    for gy in 0..image.placement.height {
        for gx in 0..image.placement.width {
            let alpha = image.data[(gy * image.placement.width + gx) as usize];
            if alpha > 0 {
                let mut c = color;
                c.set_alpha(c.alpha() * (alpha as f32 / 255.0));
                paint.set_color(c);

                let px = x + gx as i32;
                let py = y + gy as i32;

                if px >= 0
                    && py >= 0
                    && (px as u32) < pixmap.width()
                    && (py as u32) < pixmap.height()
                {
                    pixmap.fill_rect(
                        Rect::from_xywh(px as f32, py as f32, 1.0, 1.0).unwrap(),
                        &paint,
                        Transform::identity(),
                        None,
                    );
                }
            }
        }
    }
}

fn render_subpixel_glyph(pixmap: &mut PixmapMut, image: &SwashImage, x: i32, y: i32, color: Color) {
    for gy in 0..image.placement.height {
        for gx in 0..image.placement.width {
            let idx = (gy * image.placement.width + gx) as usize * 3;
            let r = image.data[idx];
            let g = image.data[idx + 1];
            let b = image.data[idx + 2];

            if r > 0 || g > 0 || b > 0 {
                let avg_alpha = (r as f32 + g as f32 + b as f32) / (3.0 * 255.0);
                let mut paint = Paint::default();
                let mut c = color;
                c.set_alpha(c.alpha() * avg_alpha);
                paint.set_color(c);

                let px = x + gx as i32;
                let py = y + gy as i32;

                if px >= 0
                    && py >= 0
                    && (px as u32) < pixmap.width()
                    && (py as u32) < pixmap.height()
                {
                    pixmap.fill_rect(
                        Rect::from_xywh(px as f32, py as f32, 1.0, 1.0).unwrap(),
                        &paint,
                        Transform::identity(),
                        None,
                    );
                }
            }
        }
    }
}

fn render_color_glyph(pixmap: &mut PixmapMut, image: &SwashImage, x: i32, y: i32) {
    for gy in 0..image.placement.height {
        for gx in 0..image.placement.width {
            let idx = (gy * image.placement.width + gx) as usize * 4;
            let r = image.data[idx];
            let g = image.data[idx + 1];
            let b = image.data[idx + 2];
            let a = image.data[idx + 3];

            if a > 0 {
                let mut paint = Paint::default();
                paint.set_color(Color::from_rgba8(r, g, b, a));

                let px = x + gx as i32;
                let py = y + gy as i32;

                if px >= 0
                    && py >= 0
                    && (px as u32) < pixmap.width()
                    && (py as u32) < pixmap.height()
                {
                    pixmap.fill_rect(
                        Rect::from_xywh(px as f32, py as f32, 1.0, 1.0).unwrap(),
                        &paint,
                        Transform::identity(),
                        None,
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tiny_skia::Rect;

    #[test]
    fn test_render_glyphs() {
        let mut pixmap_data = vec![0; 10 * 10 * 4];
        let mut pixmap = PixmapMut::from_bytes(&mut pixmap_data, 10, 10).unwrap();

        let mut image = unsafe { std::mem::MaybeUninit::<SwashImage>::uninit().assume_init() };
        unsafe {
            std::ptr::write(&mut image.content, SwashContent::Mask);
            std::ptr::write(
                &mut image.placement,
                cosmic_text::Placement {
                    left: 0,
                    top: 0,
                    width: 2,
                    height: 2,
                },
            );
            std::ptr::write(&mut image.data, vec![255, 255, 255, 255]);
        }

        render_mask_glyph(&mut pixmap, &image, 0, 0, Color::BLACK);

        let mut image_color =
            unsafe { std::mem::MaybeUninit::<SwashImage>::uninit().assume_init() };
        unsafe {
            std::ptr::write(&mut image_color.content, cosmic_text::SwashContent::Color);
            std::ptr::write(
                &mut image_color.placement,
                cosmic_text::Placement {
                    left: 0,
                    top: 0,
                    width: 1,
                    height: 1,
                },
            );
            std::ptr::write(&mut image_color.data, vec![255, 0, 0, 255]);
        }
        render_color_glyph(&mut pixmap, &image_color, 5, 5);

        let mut image_subpixel =
            unsafe { std::mem::MaybeUninit::<SwashImage>::uninit().assume_init() };
        unsafe {
            std::ptr::write(
                &mut image_subpixel.content,
                cosmic_text::SwashContent::SubpixelMask,
            );
            std::ptr::write(
                &mut image_subpixel.placement,
                cosmic_text::Placement {
                    left: 0,
                    top: 0,
                    width: 1,
                    height: 1,
                },
            );
            std::ptr::write(&mut image_subpixel.data, vec![255, 255, 255]); // RGB
        }
        render_subpixel_glyph(&mut pixmap, &image_subpixel, 2, 2, Color::WHITE);

        std::mem::forget(image);
        std::mem::forget(image_color);
        std::mem::forget(image_subpixel);
    }

    #[test]
    fn test_text_styling() {
        let color = Color::from_rgba8(255, 0, 0, 255);
        let styling = TextStyling::new(14.0, 20.0, color, "Arial".to_string());

        assert_eq!(styling.font_size(), 14.0);
        assert_eq!(styling.line_height(), 20.0);
        assert_eq!(styling.color().red(), color.red());
        assert_eq!(styling.font_family(), "Arial");
    }

    #[test]
    fn test_render_context_alignment() {
        let mut context = RenderContext::new();
        let area = Rect::from_xywh(0.0, 10.0, 100.0, 50.0).unwrap();
        let content_height = 20.0;

        context.set_vertical_alignment(VerticalAlignment::Top);
        assert_eq!(
            context.calculate_vertical_offset(area, content_height),
            10.0
        );

        context.set_vertical_alignment(VerticalAlignment::Center);
        assert_eq!(
            context.calculate_vertical_offset(area, content_height),
            10.0 + (50.0 - 20.0) / 2.0
        );

        context.set_vertical_alignment(VerticalAlignment::Bottom);
        assert_eq!(
            context.calculate_vertical_offset(area, content_height),
            10.0 + 50.0 - 20.0
        );
    }

    #[test]
    fn test_render_context_scale() {
        let mut context = RenderContext::new();
        assert_eq!(context.scale(), 1.0);
        context.set_scale(2.0);
        assert_eq!(context.scale(), 2.0);
    }

    #[test]
    fn test_font_switching() {
        let mut context = RenderContext::new();
        let text = "Hello World";
        let color = Color::BLACK;

        let mono_styling = TextStyling::new(14.0, 20.0, color, "monospace".to_string());
        let serif_styling = TextStyling::new(14.0, 20.0, color, "serif".to_string());

        let mono_width = context.measure_text(text, mono_styling);
        let serif_width = context.measure_text(text, serif_styling);

        // If font switching is working, these should likely be different
        // unless they fallback to the same font.
        // But for "monospace" and "serif", they should definitely be different on most systems.
        assert_ne!(
            mono_width, serif_width,
            "Font switching did not affect width"
        );
    }
}
