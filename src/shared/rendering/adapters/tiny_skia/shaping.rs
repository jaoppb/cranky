//! The font a run of text is shaped in, and shaping it with cosmic-text, as both the canvas and
//! the measurer do.
use super::paint::get_family;
use crate::shared::primitives::geometry::{LogicalPx, Scale};
use crate::shared::rendering::ports::font::Font;
use crate::shared::rendering::ports::text::TextFont;
use cosmic_text::{Attrs, Buffer, FontSystem, Metrics, Shaping};

/// A font family and its size in physical pixels.
pub(super) struct ShapingFont<'font> {
    family: &'font str,
    physical_size: f32,
}

impl<'font> ShapingFont<'font> {
    /// `font`, each part it leaves unset taken from `default`, sized for `scale`.
    pub(super) fn resolve(font: TextFont<'font>, default: &'font Font, scale: Scale) -> Self {
        let size = font.size().unwrap_or_else(|| default.size()).value();
        let family = font.family().unwrap_or_else(|| default.family()).as_str();
        Self {
            family,
            physical_size: LogicalPx::new(size).apply_scale(scale).value(),
        }
    }

    /// `text` shaped and laid out in this font, with a line height equal to its size.
    pub(super) fn shape(&self, font_system: &mut FontSystem, text: &str) -> Buffer {
        let metrics = Metrics::new(self.physical_size, self.physical_size * 1.0);
        let mut buffer = Buffer::new(font_system, metrics);
        let attrs = Attrs::new().family(get_family(self.family));

        buffer.set_text(text, &attrs, Shaping::Advanced, None);
        buffer.shape_until_scroll(font_system, false);
        buffer
    }
}
