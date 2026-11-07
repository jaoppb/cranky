use crate::features::layout_engine::domain::TextMeasurer;
use crate::shared::config::domain::{FontFamily, FontSize};
use crate::shared::primitives::color::{Color, DrawingColor};
use crate::shared::primitives::geometry::{Position, Scale, Size};
use crate::shared::primitives::text_align::{TextAlign, TextAlignment};
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
        TextAlignment::default(),
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
        TextAlignment::default(),
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

/// Leftmost and rightmost inked columns of a 200x40 canvas after drawing
/// "Ag" aligned within a 200px-wide box starting at x=0.
fn ink_span(align: TextAlign) -> (u32, u32) {
    let mut pixmap = Pixmap::new(200, 40).unwrap();
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
        "Ag",
        None,
        None,
        DrawingColor::Solid(Color::new(255, 255, 255, 255)),
        Position::new(0, 0),
        TextAlignment::new(align, 200),
    );
    let inked: Vec<u32> = (0..200u32)
        .filter(|&x| (0..40u32).any(|y| pixmap.pixel(x, y).is_some_and(|p| p.alpha() > 0)))
        .collect();
    (*inked.first().unwrap(), *inked.last().unwrap())
}

#[test]
fn test_draw_text_alignment_moves_ink_left_to_right() {
    let (left_min, left_max) = ink_span(TextAlign::Left);
    let (center_min, center_max) = ink_span(TextAlign::Center);
    let (right_min, right_max) = ink_span(TextAlign::Right);

    assert!(left_min < 10, "left text hugs the box's left edge");
    assert!(right_max >= 190, "right text hugs the box's right edge");
    let ink_center = u32::midpoint(center_min, center_max);
    assert!(
        (95..=105).contains(&ink_center),
        "center text is centered, ink spans {center_min}..{center_max}"
    );
    assert!(left_min < center_min && center_min < right_min);
    assert!(left_max < center_max && center_max < right_max);
}

#[test]
fn test_draw_text_wider_than_box_starts_at_origin() {
    let mut pixmap = Pixmap::new(200, 40).unwrap();
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
        "a considerably long line",
        None,
        None,
        DrawingColor::Solid(Color::new(255, 255, 255, 255)),
        Position::new(0, 0),
        TextAlignment::new(TextAlign::Center, 10),
    );
    let first_ink = (0..200u32)
        .find(|&x| (0..40u32).any(|y| pixmap.pixel(x, y).is_some_and(|p| p.alpha() > 0)));
    assert!(first_ink.is_some_and(|x| x < 10));
}
