use crate::shared::config::domain::{FontFamily, FontSize};
use crate::shared::primitives::color::{Color, DrawingColor};
use crate::shared::primitives::geometry::{LogicalPx, Position, Scale, Size};
use crate::shared::rendering::adapters::tiny_skia::canvas::TinySkiaCosmicCanvas;
use crate::shared::rendering::adapters::tiny_skia::factory::TinySkiaCanvasFactory;
use crate::shared::rendering::ports::canvas::Canvas;
use cosmic_text::{FontSystem, SwashCache};
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

    let pixel = pixmap.pixel(50, 50).unwrap();
    assert_eq!(pixel.red(), 0);
    assert_eq!(pixel.green(), 0);
    assert_eq!(pixel.blue(), 255);
    assert_eq!(pixel.alpha(), 255);
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
            LogicalPx::new(0.0),
            LogicalPx::new(2.0),
        );

        canvas.draw_border(
            Position::new(20, 20),
            Size::new(60, 60),
            DrawingColor::Solid(Color::new(0, 0, 255, 255)),
            LogicalPx::new(10.0),
            LogicalPx::new(2.0),
        );
    }

    let p0 = pixmap.pixel(0, 0).unwrap();
    assert_eq!(p0.green(), 255);
    assert_eq!(p0.alpha(), 255);

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
        LogicalPx::new(20.0),
    );
}
