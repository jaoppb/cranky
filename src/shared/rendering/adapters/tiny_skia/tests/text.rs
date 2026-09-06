use crate::features::layout_engine::domain::TextMeasurer;
use crate::shared::config::domain::{FontFamily, FontSize};
use crate::shared::primitives::color::{Color, DrawingColor};
use crate::shared::primitives::geometry::{Position, Scale, Size};
use crate::shared::rendering::adapters::tiny_skia::canvas::TinySkiaCosmicCanvas;
use crate::shared::rendering::adapters::tiny_skia::measurer::CosmicTextMeasurer;
use crate::shared::rendering::ports::canvas::Canvas;
use cosmic_text::{FontSystem, SwashCache};
use tiny_skia::Pixmap;

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

    canvas.draw_text(
        "test ",
        None,
        None,
        DrawingColor::Solid(Color::new(255, 255, 255, 255)),
        Position::new(10, 10),
    );

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
