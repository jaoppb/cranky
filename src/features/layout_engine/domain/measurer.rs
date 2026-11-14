use crate::shared::config::domain::{FontFamily, FontSize};
use crate::shared::primitives::ModuleKey;
use crate::shared::primitives::geometry::Size;

pub trait TextMeasurer: Send + Sync {
    fn measure(&mut self, text: &str, font: Option<&FontFamily>, size: Option<FontSize>) -> Size;
    fn measure_module(&self, _key: &ModuleKey) -> Option<Size> {
        None
    }
}
