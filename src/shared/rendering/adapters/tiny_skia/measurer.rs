//! Text measurement with cosmic-text, so layout sizes text with the same fonts the canvas draws it
//! with.
use super::shaping::ShapingFont;
use crate::shared::primitives::geometry::{PhysicalPx, Scale, Size};
use crate::shared::rendering::ports::font::Font;
use crate::shared::rendering::ports::text::TextFont;
use cosmic_text::FontSystem;

pub(crate) struct CosmicTextMeasurer<'a> {
    font_system: &'a mut FontSystem,
    scale: Scale,
    default_font: Font,
}

impl<'a> CosmicTextMeasurer<'a> {
    #[must_use]
    pub(crate) const fn new(
        font_system: &'a mut FontSystem,
        scale: Scale,
        default_font: Font,
    ) -> Self {
        Self {
            font_system,
            scale,
            default_font,
        }
    }
}

use crate::utils::f32_to_u32;

impl crate::features::layout_engine::domain::TextMeasurer for CosmicTextMeasurer<'_> {
    fn measure(&mut self, text: &str, font: TextFont<'_>) -> Size {
        let buffer = ShapingFont::resolve(font, &self.default_font, self.scale)
            .shape(self.font_system, text);
        let line_height = buffer.metrics().line_height;

        let mut physical_width: f32 = 0.0;
        let mut physical_height: f32 = 0.0;
        for run in buffer.layout_runs() {
            physical_width = physical_width.max(run.line_w);
            physical_height += line_height;
        }

        let width = PhysicalPx::new(physical_width).apply_inverse_scale(self.scale);
        let height = PhysicalPx::new(physical_height).apply_inverse_scale(self.scale);

        Size::new(
            f32_to_u32(width.value().ceil().max(0.0)),
            f32_to_u32(height.value().ceil().max(0.0)),
        )
    }
}
