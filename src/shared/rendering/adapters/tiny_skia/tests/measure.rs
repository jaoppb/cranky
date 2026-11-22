//! How the measurer turns shaped lines into a logical size.
use crate::features::layout_engine::domain::TextMeasurer;
use crate::shared::config::domain::{FontFamily, FontSize};
use crate::shared::primitives::geometry::{Scale, Size};
use crate::shared::rendering::adapters::tiny_skia::measurer::CosmicTextMeasurer;
use crate::shared::rendering::ports::font::Font;
use crate::shared::rendering::ports::text::TextFont;
use cosmic_text::FontSystem;

fn measure(font_system: &mut FontSystem, scale: f32, text: &str) -> Size {
    let font = Font::new(
        FontFamily::new("sans-serif".to_string()),
        FontSize::new(14.0),
    );
    CosmicTextMeasurer::new(font_system, Scale::new(scale), font).measure(text, TextFont::default())
}

#[test]
fn measurer_takes_the_widest_line_and_stacks_the_line_heights() {
    let mut font_system = FontSystem::new();
    let one_line = measure(&mut font_system, 1.0, "HgHg");
    let two_lines = measure(&mut font_system, 1.0, "Hg\nHgHg");
    assert_eq!(two_lines.width(), one_line.width());
    assert_eq!(two_lines.height(), 2 * one_line.height());
}

#[test]
fn measurer_reports_widths_in_logical_pixels() {
    let mut font_system = FontSystem::new();
    let narrow = measure(&mut font_system, 1.0, "Hg").width();
    let wide = measure(&mut font_system, 1.0, "HgHgHg").width();
    let scaled = measure(&mut font_system, 2.0, "HgHgHg").width();
    assert!(wide > 2 * narrow, "three times the text {wide} vs {narrow}");
    assert!(
        scaled.abs_diff(wide) <= 1,
        "scale 2 width {scaled} vs scale 1 width {wide}"
    );
}
