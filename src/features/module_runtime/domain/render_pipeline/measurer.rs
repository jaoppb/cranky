//! A text measurer that also answers embedded child modules' sizes, from the sizes those children
//! last reported.
use crate::features::layout_engine::domain::TextMeasurer;
use crate::shared::primitives::geometry::Size;
use crate::shared::primitives::{ChildSizesMap, ModuleKey};
use crate::shared::rendering::ports::text::TextFont;

pub(crate) struct ModuleSizeMeasurer<'a, M: TextMeasurer> {
    inner: M,
    child_sizes: Option<&'a ChildSizesMap>,
}

impl<'a, M: TextMeasurer> ModuleSizeMeasurer<'a, M> {
    #[must_use]
    pub(crate) const fn new(inner: M, child_sizes: Option<&'a ChildSizesMap>) -> Self {
        Self { inner, child_sizes }
    }
}

impl<M: TextMeasurer> TextMeasurer for ModuleSizeMeasurer<'_, M> {
    fn measure(&mut self, text: &str, font: TextFont<'_>) -> Size {
        self.inner.measure(text, font)
    }

    fn measure_module(&self, key: &ModuleKey) -> Option<Size> {
        let size = self.child_sizes.and_then(|sizes| {
            sizes
                .get_by_name_or_key(key.name(), key.instance_id())
                .copied()
        });
        tracing::trace!(
            ?key,
            ?size,
            has_child_sizes = self.child_sizes.is_some(),
            "measure_module called"
        );
        size
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::config::domain::{FontFamily, FontSize};

    /// Reports the font it was asked for: the family name's length wide and the size tall.
    struct FontEcho;
    impl TextMeasurer for FontEcho {
        fn measure(&mut self, text: &str, font: TextFont<'_>) -> Size {
            let family = font.family().map_or(0, |family| family.as_str().len());
            let width = family.saturating_mul(100).saturating_add(text.len());
            Size::new(
                u32::try_from(width).unwrap(),
                crate::utils::f32_to_u32(font.size().map_or(0.0, FontSize::value)),
            )
        }
    }

    #[test]
    fn test_measure_forwards_the_text_and_its_font_to_the_inner_measurer() {
        let mut measurer = ModuleSizeMeasurer::new(FontEcho, None);
        let family = FontFamily::new("serif".to_string());
        assert_eq!(
            measurer.measure(
                "ab",
                TextFont::new(Some(&family), Some(FontSize::new(19.0)))
            ),
            Size::new(502, 19)
        );
    }
}
