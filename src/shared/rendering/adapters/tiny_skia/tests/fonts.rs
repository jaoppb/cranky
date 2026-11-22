//! How the canvas, the measurer and the factory use their target, scale and default font.
use super::target;
use crate::features::layout_engine::domain::TextMeasurer;
use crate::shared::config::domain::{FontFamily, FontSize};
use crate::shared::primitives::TextAlignment;
use crate::shared::primitives::color::{Color, DrawingColor};
use crate::shared::primitives::geometry::Rect;
use crate::shared::primitives::geometry::{LogicalPx, Position, Scale, Size};
use crate::shared::rendering::adapters::tiny_skia::canvas::TinySkiaCosmicCanvas;
use crate::shared::rendering::adapters::tiny_skia::factory::FontCache;
use crate::shared::rendering::adapters::tiny_skia::factory::TinySkiaCanvasFactory;
use crate::shared::rendering::adapters::tiny_skia::measurer::CosmicTextMeasurer;
use crate::shared::rendering::ports::canvas::{Canvas, CanvasFactory};
use crate::shared::rendering::ports::draw::{BorderStroke, PaintTarget, RgbaImage, RoundedRect};
use crate::shared::rendering::ports::font::Font;
use crate::shared::rendering::ports::text::{TextFont, TextPlacement, TextRun};
use cosmic_text::FontSystem;
use tiny_skia::{Pixmap, PixmapRef};

fn sans() -> FontFamily {
    FontFamily::new("sans-serif".to_string())
}

/// The inked bounding box `[left, top, right, bottom]` of `text` drawn at `position` on a 200x80
/// canvas whose default size is `default_size`, with an optional size override.
fn ink_box(default_size: f32, size: Option<FontSize>, position: Position) -> [u32; 4] {
    let mut pixmap = Pixmap::new(200, 80).unwrap();
    let mut fonts = FontCache::new();
    {
        let mut canvas = TinySkiaCosmicCanvas::new(
            target(&mut pixmap, 1.0),
            &mut fonts,
            Font::new(sans(), FontSize::new(default_size)),
        );
        canvas.draw_text(
            TextRun::new(
                "Hg",
                TextFont::new(None, size),
                DrawingColor::Solid(Color::rgb(255, 0, 0)),
            ),
            TextPlacement::new(position, TextAlignment::default()),
        );
    }
    let inked: Vec<(u32, u32)> = (0..80u32)
        .flat_map(|y| (0..200u32).map(move |x| (x, y)))
        .filter(|&(x, y)| pixmap.pixel(x, y).is_some_and(|pixel| pixel.alpha() > 0))
        .collect();
    for &(x, y) in &inked {
        let pixel = pixmap.pixel(x, y).unwrap();
        assert_eq!(pixel.red(), 0, "red text lands in the blue channel");
    }
    let columns = inked.iter().map(|&(x, _)| x);
    let rows = inked.iter().map(|&(_, y)| y);
    [
        columns.clone().min().unwrap(),
        rows.clone().min().unwrap(),
        columns.max().unwrap(),
        rows.max().unwrap(),
    ]
}

#[test]
fn text_starts_at_its_position() {
    let [left, top, ..] = ink_box(14.0, None, Position::new(60, 10));
    assert!((60..70).contains(&left), "ink starts at x {left}");
    assert!((10..20).contains(&top), "ink starts at y {top}");
}

#[test]
fn text_uses_the_default_size_unless_its_own_is_given() {
    let height = |[_, top, _, bottom]: [u32; 4]| bottom - top;
    let small = height(ink_box(14.0, None, Position::new(0, 0)));
    let large = height(ink_box(28.0, None, Position::new(0, 0)));
    let overridden = height(ink_box(
        14.0,
        Some(FontSize::new(28.0)),
        Position::new(0, 0),
    ));
    assert!(large > small + 5, "28px ink {large} vs 14px ink {small}");
    assert_eq!(overridden, large);
}

#[test]
fn measurer_sizes_text_in_logical_pixels_with_its_default_size() {
    let mut font_system = FontSystem::new();
    let mut measure = |scale: f32, size: f32| {
        CosmicTextMeasurer::new(
            &mut font_system,
            Scale::new(scale),
            Font::new(sans(), FontSize::new(size)),
        )
        .measure("Hg", TextFont::default())
    };
    assert_eq!(measure(1.0, 14.0).height(), 14);
    assert_eq!(measure(2.0, 14.0).height(), 14);
    assert_eq!(measure(1.0, 28.0).height(), 28);
}

#[test]
fn measurer_sizes_text_in_the_size_it_is_asked_for_over_its_default() {
    let mut font_system = FontSystem::new();
    let mut measurer = CosmicTextMeasurer::new(
        &mut font_system,
        Scale::new(2.0),
        Font::new(sans(), FontSize::new(14.0)),
    );
    let family = sans();
    assert_eq!(
        measurer
            .measure(
                "Hg",
                TextFont::new(Some(&family), Some(FontSize::new(23.0)))
            )
            .height(),
        23
    );
}

#[test]
fn factory_canvas_paints_rows_of_its_size_at_its_scale() {
    let mut factory = TinySkiaCanvasFactory::new();
    let mut data = vec![0; 60 * 20 * 4];
    {
        let mut canvas = factory.create_canvas(
            PaintTarget::new(&mut data, Size::new(60, 20), Scale::new(2.0)),
            Font::new(sans(), FontSize::new(14.0)),
        );
        canvas.draw_rect(
            RoundedRect::new(
                Rect::new(Position::new(0, 0), Size::new(10, 5)),
                LogicalPx::new(0.0),
            ),
            DrawingColor::Solid(Color::rgb(255, 0, 0)),
        );
    }
    let pixmap = PixmapRef::from_bytes(&data, 60, 20).unwrap();
    let inked = |x, y| pixmap.pixel(x, y).unwrap().alpha() > 0;
    assert!(inked(19, 9) && !inked(20, 9) && !inked(19, 10));
}

#[test]
fn factory_measurer_uses_its_scale_and_default_size() {
    let mut factory = TinySkiaCanvasFactory::new();
    let size = factory
        .create_text_measurer(Scale::new(2.0), Font::new(sans(), FontSize::new(28.0)))
        .measure("Hg", TextFont::default());
    assert_eq!(size.height(), 28);
}

#[test]
fn canvas_over_a_buffer_of_the_wrong_size_draws_nothing() {
    let mut factory = TinySkiaCanvasFactory::new();
    let mut data = vec![0; 10];
    {
        let mut canvas = factory.create_canvas(
            PaintTarget::new(&mut data, Size::new(60, 20), Scale::new(1.0)),
            Font::new(sans(), FontSize::new(14.0)),
        );
        let color = DrawingColor::Solid(Color::rgb(255, 0, 0));
        canvas.draw_rect(
            RoundedRect::new(
                Rect::new(Position::new(0, 0), Size::new(10, 5)),
                LogicalPx::new(0.0),
            ),
            color.clone(),
        );
        canvas.draw_border(
            RoundedRect::new(
                Rect::new(Position::new(0, 0), Size::new(10, 5)),
                LogicalPx::new(0.0),
            ),
            BorderStroke::new(color.clone(), LogicalPx::new(1.0)),
        );
        canvas.draw_text(
            TextRun::new("x", TextFont::new(None, None), color),
            TextPlacement::new(Position::new(0, 0), TextAlignment::default()),
        );
        canvas.draw_image(
            RgbaImage::new(&[255, 0, 0, 255], Size::new(1, 1)),
            Rect::new(Position::new(0, 0), Size::new(4, 4)),
        );
    }
    assert_eq!(data, vec![0; 10]);
}
