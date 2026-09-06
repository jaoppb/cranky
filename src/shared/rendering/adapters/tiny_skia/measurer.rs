use super::paint::get_family;
use crate::shared::config::domain::{FontFamily, FontSize};
use crate::shared::primitives::geometry::{LogicalPx, PhysicalPx, Scale, Size};
use cosmic_text::{Attrs, Buffer, FontSystem, Metrics, Shaping};

pub struct CosmicTextMeasurer<'a> {
    font_system: &'a mut FontSystem,
    scale: Scale,
    default_font_family: FontFamily,
    default_font_size: FontSize,
}

impl<'a> CosmicTextMeasurer<'a> {
    #[must_use]
    pub const fn new(
        font_system: &'a mut FontSystem,
        scale: Scale,
        default_font_family: FontFamily,
        default_font_size: FontSize,
    ) -> Self {
        Self {
            font_system,
            scale,
            default_font_family,
            default_font_size,
        }
    }
}

impl crate::features::layout_engine::domain::TextMeasurer for CosmicTextMeasurer<'_> {
    #[allow(
        clippy::as_conversions,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss
    )]
    fn measure(
        &mut self,
        text: &str,
        font_family: Option<&FontFamily>,
        font_size: Option<FontSize>,
    ) -> Size {
        let size = font_size.unwrap_or(self.default_font_size).value();
        let family = font_family.unwrap_or(&self.default_font_family).as_str();

        let physical_size = LogicalPx::new(size).apply_scale(&self.scale).value();
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

        let w = PhysicalPx::new(physical_width).apply_inverse_scale(&self.scale);
        let h = PhysicalPx::new(physical_height).apply_inverse_scale(&self.scale);

        Size::new(
            w.value().ceil().max(0.0) as u32,
            h.value().ceil().max(0.0) as u32,
        )
    }
}
