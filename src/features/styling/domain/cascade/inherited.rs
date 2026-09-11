use super::super::computed_style::ComputedStyle;
use crate::shared::config::domain::FontSize;
use crate::shared::primitives::color::DrawingColor;
use std::sync::Arc;

/// The subset of computed style that CSS actually inherits down the element
/// tree: `color`, `accent-color`, `font-family`, `font-size`.
///
/// Cheap to pass down unconditionally — `DrawingColor`/`FontSize` are small
/// and `font_family` is reference-counted — so descending through elements
/// that redeclare none of these costs only a few copies and a refcount
/// bump, never the deep clone a full `ComputedStyle` would need.
#[derive(Debug, Clone, Default)]
pub struct InheritedStyle {
    color: Option<DrawingColor>,
    accent_color: Option<DrawingColor>,
    font_family: Option<Arc<str>>,
    font_size: Option<FontSize>,
}

impl InheritedStyle {
    #[must_use]
    pub const fn color(&self) -> Option<&DrawingColor> {
        self.color.as_ref()
    }

    #[must_use]
    pub const fn accent_color(&self) -> Option<&DrawingColor> {
        self.accent_color.as_ref()
    }

    #[must_use]
    pub const fn font_family(&self) -> Option<&Arc<str>> {
        self.font_family.as_ref()
    }

    #[must_use]
    pub const fn font_size(&self) -> Option<&FontSize> {
        self.font_size.as_ref()
    }

    /// Builds the context a node's children inherit from: whatever this
    /// node's own computed style ended up with, after its own cascade —
    /// inheritance from further up included — was applied.
    #[must_use]
    pub fn from_computed(style: &ComputedStyle) -> Self {
        Self {
            color: style.color().cloned(),
            accent_color: style.accent_color().cloned(),
            font_family: style.font_family().map(|f| Arc::from(f.as_str())),
            font_size: style.font_size(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::config::domain::FontFamily;

    #[test]
    fn test_from_computed_extracts_only_inheritable_fields() {
        let mut style = ComputedStyle::default();
        style.set_color(DrawingColor::parse("#ffffff").unwrap());
        style.set_font_family(FontFamily::new("Mono".to_string()));
        style.set_font_size(FontSize::new(14.0));

        let inherited = InheritedStyle::from_computed(&style);
        assert_eq!(
            inherited.color(),
            Some(&DrawingColor::parse("#ffffff").unwrap())
        );
        assert_eq!(inherited.font_family().map(AsRef::as_ref), Some("Mono"));
        assert_eq!(inherited.font_size().map(FontSize::value), Some(14.0));
        assert_eq!(inherited.accent_color(), None);
    }
}
