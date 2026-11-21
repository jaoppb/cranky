//! The measuring hook a layout pass calls for leaves whose size depends on content.
use crate::shared::config::domain::{FontFamily, FontSize};
use crate::shared::primitives::ModuleKey;
use crate::shared::primitives::geometry::Size;

/// Supplies the intrinsic sizes of leaves the layout engine cannot size itself: text runs and
/// embedded child modules.
pub trait TextMeasurer: Send + Sync {
    /// Size in logical pixels of `text` set in `font` at `size`, or the defaults when `None`.
    fn measure(&mut self, text: &str, font: Option<&FontFamily>, size: Option<FontSize>) -> Size;
    /// Last size the embedded module `key` reported, or `None` when it has not reported one yet.
    fn measure_module(&self, _key: &ModuleKey) -> Option<Size> {
        None
    }
}
