use super::target;
use crate::features::layout_engine::domain::TextMeasurer;
use crate::shared::config::domain::{FontFamily, FontSize};
use crate::shared::primitives::color::{Color, DrawingColor};
use crate::shared::primitives::geometry::Rect;
use crate::shared::primitives::geometry::{Position, Scale, Size};
use crate::shared::primitives::{TextAlign, TextAlignment};
use crate::shared::rendering::adapters::tiny_skia::canvas::TinySkiaCosmicCanvas;
use crate::shared::rendering::adapters::tiny_skia::factory::FontCache;
use crate::shared::rendering::adapters::tiny_skia::measurer::CosmicTextMeasurer;
use crate::shared::rendering::ports::canvas::Canvas;
use crate::shared::rendering::ports::draw::RgbaImage;
use crate::shared::rendering::ports::font::Font;
use crate::shared::rendering::ports::text::{TextFont, TextPlacement, TextRun};
use cosmic_text::FontSystem;
use tiny_skia::Pixmap;

#[test]
fn test_canvas_measure_text() {
    let mut font_system = FontSystem::new();

    let mut measurer = CosmicTextMeasurer::new(
        &mut font_system,
        Scale::new(1.0),
        Font::new(
            FontFamily::new("sans-serif".to_string()),
            FontSize::new(14.0),
        ),
    );

    let size = measurer.measure("test", TextFont::default());
    assert!(size.width() > 0);
    assert!(size.height() > 0);
}

#[test]
fn test_canvas_draw_text() {
    let mut pixmap = Pixmap::new(100, 100).unwrap();
    let mut fonts = FontCache::new();

    let mut canvas = TinySkiaCosmicCanvas::new(
        target(&mut pixmap, 1.0),
        &mut fonts,
        Font::new(
            FontFamily::new("sans-serif".to_string()),
            FontSize::new(14.0),
        ),
    );

    canvas.draw_text(
        TextRun::new(
            "test ",
            TextFont::new(None, None),
            DrawingColor::Solid(Color::rgb(255, 255, 255)),
        ),
        TextPlacement::new(Position::new(10, 10), TextAlignment::default()),
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
fn test_canvas_draw_image() {
    let mut pixmap = Pixmap::new(100, 100).unwrap();
    let mut fonts = FontCache::new();

    let mut canvas = TinySkiaCosmicCanvas::new(
        target(&mut pixmap, 1.0),
        &mut fonts,
        Font::new(
            FontFamily::new("sans-serif".to_string()),
            FontSize::new(14.0),
        ),
    );

    let image_data = vec![
        255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 0, 255,
    ];

    canvas.draw_image(
        RgbaImage::new(&image_data, Size::new(2, 2)),
        Rect::new(Position::new(10, 10), Size::new(20, 20)),
    );
}

/// Leftmost and rightmost inked columns of a 200x40 canvas after drawing
/// "Ag" aligned within a 200px-wide box starting at x=0.
fn ink_span(align: TextAlign) -> (u32, u32) {
    let mut pixmap = Pixmap::new(200, 40).unwrap();
    let mut fonts = FontCache::new();
    let mut canvas = TinySkiaCosmicCanvas::new(
        target(&mut pixmap, 1.0),
        &mut fonts,
        Font::new(
            FontFamily::new("sans-serif".to_string()),
            FontSize::new(14.0),
        ),
    );
    canvas.draw_text(
        TextRun::new(
            "Ag",
            TextFont::new(None, None),
            DrawingColor::Solid(Color::rgb(255, 255, 255)),
        ),
        TextPlacement::new(Position::new(0, 0), TextAlignment::new(align, 200)),
    );
    let inked: Vec<u32> = (0..200u32)
        .filter(|&x| (0..40u32).any(|y| pixmap.pixel(x, y).is_some_and(|pixel| pixel.alpha() > 0)))
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
    let mut fonts = FontCache::new();
    let mut canvas = TinySkiaCosmicCanvas::new(
        target(&mut pixmap, 1.0),
        &mut fonts,
        Font::new(
            FontFamily::new("sans-serif".to_string()),
            FontSize::new(14.0),
        ),
    );
    canvas.draw_text(
        TextRun::new(
            "a considerably long line",
            TextFont::new(None, None),
            DrawingColor::Solid(Color::rgb(255, 255, 255)),
        ),
        TextPlacement::new(
            Position::new(0, 0),
            TextAlignment::new(TextAlign::Center, 10),
        ),
    );
    let first_ink = (0..200u32)
        .find(|&x| (0..40u32).any(|y| pixmap.pixel(x, y).is_some_and(|pixel| pixel.alpha() > 0)));
    assert!(first_ink.is_some_and(|x| x < 10));
}
