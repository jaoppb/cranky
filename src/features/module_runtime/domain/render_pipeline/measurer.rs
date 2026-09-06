use crate::features::layout_engine::domain::TextMeasurer;
use crate::shared::config::domain::{FontFamily, FontSize};
use crate::shared::primitives::geometry::Size;
use crate::shared::primitives::{ChildSizesMap, ModuleKey};

pub struct ModuleSizeMeasurer<'a, M: TextMeasurer> {
    inner: M,
    child_sizes: Option<&'a ChildSizesMap>,
}

impl<'a, M: TextMeasurer> ModuleSizeMeasurer<'a, M> {
    #[must_use]
    pub const fn new(inner: M, child_sizes: Option<&'a ChildSizesMap>) -> Self {
        Self { inner, child_sizes }
    }
}

impl<M: TextMeasurer> TextMeasurer for ModuleSizeMeasurer<'_, M> {
    fn measure(&mut self, text: &str, font: Option<&FontFamily>, size: Option<FontSize>) -> Size {
        self.inner.measure(text, font, size)
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
