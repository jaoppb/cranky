//! Text measurement with cosmic-text, so layout sizes text with the same fonts the canvas draws it
//! with.
use super::paint::get_family;
use crate::shared::primitives::geometry::{LogicalPx, PhysicalPx, Scale, Size};
use crate::shared::rendering::ports::font::Font;
use crate::shared::rendering::ports::text::TextFont;
use cosmic_text::{Attrs, Buffer, FontSystem, Metrics, Shaping};

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
        let size = font
            .size()
            .unwrap_or_else(|| self.default_font.size())
            .value();
        let family = font
            .family()
            .unwrap_or_else(|| self.default_font.family())
            .as_str();

        let physical_size = LogicalPx::new(size).apply_scale(self.scale).value();
        let metrics = Metrics::new(physical_size, physical_size * 1.0);
        let mut buffer = Buffer::new(self.font_system, metrics);
        let attrs = Attrs::new().family(get_family(family));

        buffer.set_text(text, &attrs, Shaping::Advanced, None);
        buffer.shape_until_scroll(self.font_system, false);

        let mut physical_width: f32 = 0.0;
        let mut physical_height: f32 = 0.0;
        for run in buffer.layout_runs() {
            physical_width = physical_width.max(run.line_w);
            physical_height += metrics.line_height;
        }

        let width = PhysicalPx::new(physical_width).apply_inverse_scale(self.scale);
        let height = PhysicalPx::new(physical_height).apply_inverse_scale(self.scale);

        Size::new(
            f32_to_u32(width.value().ceil().max(0.0)),
            f32_to_u32(height.value().ceil().max(0.0)),
        )
    }
}
