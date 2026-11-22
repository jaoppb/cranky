//! Where the tiny-skia canvas puts each shape: position, size, radius and scale.
use super::target;
use crate::shared::config::domain::{FontFamily, FontSize};
use crate::shared::primitives::color::{Color, DrawingColor};
use crate::shared::primitives::geometry::Rect;
use crate::shared::primitives::geometry::{LogicalPx, Position, Size};
use crate::shared::rendering::adapters::tiny_skia::canvas::TinySkiaCosmicCanvas;
use crate::shared::rendering::adapters::tiny_skia::factory::FontCache;
use crate::shared::rendering::ports::canvas::Canvas;
use crate::shared::rendering::ports::draw::{BorderStroke, RgbaImage, RoundedRect};
use crate::shared::rendering::ports::font::Font;
use tiny_skia::Pixmap;

fn red() -> DrawingColor {
    DrawingColor::Solid(Color::rgb(255, 0, 0))
}

/// Draws with `draw` on a fresh square canvas `side` pixels wide at `scale`, and returns the
/// pixmap.
fn paint(side: u32, scale: f32, draw: impl FnOnce(&mut dyn Canvas)) -> Pixmap {
    let mut pixmap = Pixmap::new(side, side).unwrap();
    let mut fonts = FontCache::new();
    {
        let mut canvas = TinySkiaCosmicCanvas::new(
            target(&mut pixmap, scale),
            &mut fonts,
            Font::new(
                FontFamily::new("sans-serif".to_string()),
                FontSize::new(14.0),
            ),
        );
        draw(&mut canvas);
    }
    pixmap
}

fn inked(pixmap: &Pixmap, x: u32, y: u32) -> bool {
    pixmap.pixel(x, y).unwrap().alpha() > 0
}

#[test]
fn rect_covers_exactly_its_area() {
    let pixmap = paint(100, 1.0, |canvas| {
        canvas.draw_rect(
            RoundedRect::new(
                Rect::new(Position::new(10, 20), Size::new(30, 50)),
                LogicalPx::new(0.0),
            ),
            red(),
        );
    });
    assert!(inked(&pixmap, 10, 20) && inked(&pixmap, 39, 69));
    assert!(!inked(&pixmap, 9, 25) && !inked(&pixmap, 15, 19));
    assert!(!inked(&pixmap, 40, 25) && !inked(&pixmap, 15, 70));
    assert!(inked(&pixmap, 15, 65), "the rect is taller than it is wide");
}

#[test]
fn rect_is_scaled_to_physical_pixels() {
    let pixmap = paint(200, 2.0, |canvas| {
        canvas.draw_rect(
            RoundedRect::new(
                Rect::new(Position::new(10, 20), Size::new(30, 50)),
                LogicalPx::new(0.0),
            ),
            red(),
        );
    });
    assert!(inked(&pixmap, 20, 40) && inked(&pixmap, 79, 139));
    assert!(!inked(&pixmap, 19, 40) && !inked(&pixmap, 20, 39));
    assert!(!inked(&pixmap, 80, 100) && !inked(&pixmap, 50, 140));
}

#[test]
fn rect_radius_rounds_the_corners() {
    let draw = |radius: f32| {
        paint(100, 1.0, |canvas| {
            canvas.draw_rect(
                RoundedRect::new(
                    Rect::new(Position::new(10, 20), Size::new(40, 30)),
                    LogicalPx::new(radius),
                ),
                red(),
            );
        })
    };
    let rounded = draw(10.0);
    assert!(!inked(&rounded, 10, 20) && !inked(&rounded, 49, 49));
    assert!(inked(&rounded, 30, 35));
    let square = draw(0.0);
    assert!(inked(&square, 10, 20) && inked(&square, 49, 49));
}

#[test]
fn border_strokes_the_edges_of_its_area() {
    let pixmap = paint(100, 1.0, |canvas| {
        canvas.draw_border(
            RoundedRect::new(
                Rect::new(Position::new(10, 20), Size::new(40, 30)),
                LogicalPx::new(0.0),
            ),
            BorderStroke::new(red(), LogicalPx::new(2.0)),
        );
    });
    for (x, y) in [(10, 35), (49, 35), (30, 20), (30, 49), (10, 20), (49, 49)] {
        assert!(inked(&pixmap, x, y), "edge pixel ({x}, {y})");
    }
    for (x, y) in [(30, 35), (9, 35), (50, 35), (30, 19), (30, 50)] {
        assert!(!inked(&pixmap, x, y), "pixel ({x}, {y}) off the border");
    }
}

#[test]
fn border_radius_rounds_and_zero_width_draws_nothing() {
    let rounded = paint(100, 1.0, |canvas| {
        canvas.draw_border(
            RoundedRect::new(
                Rect::new(Position::new(10, 20), Size::new(40, 30)),
                LogicalPx::new(10.0),
            ),
            BorderStroke::new(red(), LogicalPx::new(2.0)),
        );
    });
    assert!(!inked(&rounded, 10, 20) && inked(&rounded, 30, 20));

    let hairline = paint(100, 1.0, |canvas| {
        canvas.draw_border(
            RoundedRect::new(
                Rect::new(Position::new(10, 20), Size::new(40, 30)),
                LogicalPx::new(0.0),
            ),
            BorderStroke::new(red(), LogicalPx::new(0.0)),
        );
    });
    assert!(hairline.pixels().iter().all(|pixel| pixel.alpha() == 0));
}

#[test]
fn image_is_stretched_from_its_pixels_over_its_logical_rect() {
    // Two texels, red then green, stretched over 20x10 logical pixels at (30, 40).
    let image_data = [255, 0, 0, 255, 0, 255, 0, 255];
    let pixmap = paint(100, 1.0, |canvas| {
        canvas.draw_image(
            RgbaImage::new(&image_data, Size::new(2, 1)),
            Rect::new(Position::new(30, 40), Size::new(20, 10)),
        );
    });
    let left = pixmap.pixel(31, 45).unwrap();
    let right = pixmap.pixel(48, 45).unwrap();
    // RGBA input lands as BGRA, so red reads back in the blue channel.
    assert_eq!((left.blue(), left.green()), (255, 0));
    assert_eq!((right.blue(), right.green()), (0, 255));
    assert!(inked(&pixmap, 31, 49) && inked(&pixmap, 30, 40));
    assert!(!inked(&pixmap, 31, 50) && !inked(&pixmap, 50, 45));
    assert!(!inked(&pixmap, 29, 45) && !inked(&pixmap, 31, 39));
}

#[test]
fn image_position_and_size_are_scaled() {
    let image_data = [255, 0, 0, 255, 0, 255, 0, 255];
    let pixmap = paint(200, 2.0, |canvas| {
        canvas.draw_image(
            RgbaImage::new(&image_data, Size::new(2, 1)),
            Rect::new(Position::new(30, 40), Size::new(20, 10)),
        );
    });
    assert!(inked(&pixmap, 61, 81) && inked(&pixmap, 98, 98));
    assert!(!inked(&pixmap, 59, 90) && !inked(&pixmap, 70, 79));
    assert!(!inked(&pixmap, 101, 90) && !inked(&pixmap, 70, 101));
}
