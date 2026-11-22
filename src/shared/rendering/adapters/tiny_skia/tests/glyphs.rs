//! Pins how glyphs are composited: gradient text, and the alpha of fully covered pixels.
use super::sans_canvas;
use crate::shared::primitives::TextAlignment;
use crate::shared::primitives::color::{Color, DrawingColor};
use crate::shared::primitives::geometry::Position;
use crate::shared::rendering::adapters::tiny_skia::factory::FontCache;
use crate::shared::rendering::ports::canvas::Canvas;
use crate::shared::rendering::ports::text::{TextFont, TextPlacement, TextRun};
use tiny_skia::Pixmap;

/// A 100x100 pixmap after drawing "MW" at `origin` in a red-to-green gradient at `angle`.
fn gradient_text(origin: Position, angle: f32) -> Pixmap {
    let mut pixmap = Pixmap::new(100, 100).unwrap();
    let mut fonts = FontCache::new();
    let mut canvas = sans_canvas(&mut pixmap, &mut fonts);
    canvas.draw_text(
        TextRun::new(
            "MW",
            TextFont::new(None, None),
            DrawingColor::Gradient(vec![Color::rgb(255, 0, 0), Color::rgb(0, 255, 0)], angle),
        ),
        TextPlacement::new(origin, TextAlignment::default()),
    );
    drop(canvas);
    pixmap
}

/// The inked pixels of `pixmap` that carry some of the gradient's green end.
fn green_ink(pixmap: &Pixmap) -> usize {
    pixmap
        .pixels()
        .iter()
        .filter(|pixel| pixel.alpha() > 0 && pixel.green() > 0)
        .count()
}

/// The red stop of the gradient, after the canvas swaps red and blue.
fn is_red_ink(pixel: tiny_skia::PremultipliedColorU8) -> bool {
    pixel.alpha() > 0 && pixel.green() == 0 && pixel.red() == 0 && pixel.blue() == pixel.alpha()
}

/// Pins today's behaviour, which is a bug: each glyph's gradient is laid over the glyph's box
/// on the canvas but sampled in the glyph's own pixmap, which starts at 0. Text at the origin
/// shows both ends of the gradient; in text drawn well away from it, every pixel of a glyph's
/// pixmap lies before the start of its gradient, so the glyph paints only the first color.
#[test]
fn test_canvas_draw_text_gradient() {
    for angle in [0.0, 90.0] {
        let at_origin = gradient_text(Position::new(0, 0), angle);
        assert!(green_ink(&at_origin) > 0, "no green ink at {angle}deg");

        let away = gradient_text(Position::new(40, 40), angle);
        let inked: Vec<_> = away
            .pixels()
            .iter()
            .filter(|pixel| pixel.alpha() > 0)
            .collect();
        assert!(!inked.is_empty(), "no ink at {angle}deg");
        assert!(
            inked.iter().all(|&&pixel| is_red_ink(pixel)),
            "text away from the origin is not all red at {angle}deg"
        );
    }
}

/// Coverage is scaled by 255, so a pixel the glyph covers fully keeps the paint's full alpha.
#[test]
fn test_canvas_draw_text_covers_whole_pixels_opaquely() {
    let mut pixmap = Pixmap::new(100, 100).unwrap();
    let mut fonts = FontCache::new();
    let mut canvas = sans_canvas(&mut pixmap, &mut fonts);
    canvas.draw_text(
        TextRun::new(
            "MW",
            TextFont::new(None, None),
            DrawingColor::Solid(Color::rgb(255, 255, 255)),
        ),
        TextPlacement::new(Position::new(10, 10), TextAlignment::default()),
    );
    drop(canvas);
    assert!(pixmap.pixels().iter().any(|pixel| pixel.alpha() == 255));
}
