use super::super::computed_style::ComputedStyle;
use crate::shared::config::domain::FontSize;
use crate::shared::primitives::color::DrawingColor;
use std::collections::HashMap;
use std::sync::Arc;

/// The default `rem` base and the fallback used for `em` when no ancestor
/// has ever declared a font size. Matches this renderer's own established
/// default (`layout_phase.rs`/`paint_phase.rs` already fall back to 14px
/// when `ComputedStyle::font_size` is `None`), not CSS's spec default of
/// 16px — using 16 here would leave the two defaults disagreeing.
pub(crate) const DEFAULT_FONT_SIZE: f32 = 14.0;

/// The subset of computed style that CSS actually inherits down the element
/// tree: `color`, `accent-color`, `font-family`, `font-size`.
///
/// Plus custom properties (`--name`, inherited by default, same as real
/// CSS) and the fixed `rem` base — the latter isn't inherited in the CSS
/// sense (it never changes as it descends) but travels the same path for
/// the same reason.
///
/// Cheap to pass down unconditionally — `DrawingColor`/`FontSize` are small,
/// `font_family` is reference-counted, and `custom_properties` is shared via
/// `Arc` and only cloned when an element actually declares one — so
/// descending through elements that redeclare none of these costs only a
/// few copies and refcount bumps, never the deep clone a full
/// `ComputedStyle` would need.
#[derive(Debug, Clone)]
pub struct InheritedStyle {
    color: Option<DrawingColor>,
    accent_color: Option<DrawingColor>,
    font_family: Option<Arc<str>>,
    font_size: Option<FontSize>,
    root_font_size: FontSize,
    custom_properties: Arc<HashMap<String, String>>,
}

impl Default for InheritedStyle {
    fn default() -> Self {
        Self {
            color: None,
            accent_color: None,
            font_family: None,
            font_size: None,
            root_font_size: FontSize::new(DEFAULT_FONT_SIZE),
            custom_properties: Arc::new(HashMap::new()),
        }
    }
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

    /// The size `1rem` resolves against — fixed for the whole tree
    /// (`DEFAULT_FONT_SIZE`, since discovering it from the root element's
    /// own declared font-size would need a two-pass resolution this
    /// renderer doesn't do).
    #[must_use]
    pub const fn root_font_size(&self) -> FontSize {
        self.root_font_size
    }

    #[must_use]
    pub const fn custom_properties(&self) -> &Arc<HashMap<String, String>> {
        &self.custom_properties
    }

    /// Builds the context this node's children inherit from: whatever this
    /// node's own computed style ended up with, after its own cascade —
    /// inheritance from further up included — was applied, plus the fully
    /// resolved custom-property map `collapse` produced for this element.
    /// `root_font_size` carries forward unchanged, since it's fixed for the
    /// whole tree, not actually inherited element-to-element.
    #[must_use]
    pub fn descend(
        &self,
        style: &ComputedStyle,
        custom_properties: Arc<HashMap<String, String>>,
    ) -> Self {
        Self {
            color: style.color().cloned(),
            accent_color: style.accent_color().cloned(),
            font_family: style.font_family().map(|f| Arc::from(f.as_str())),
            font_size: style.font_size(),
            root_font_size: self.root_font_size,
            custom_properties,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::config::domain::FontFamily;

    #[test]
    fn test_descend_extracts_only_inheritable_fields() {
        let mut style = ComputedStyle::default();
        style.set_color(DrawingColor::parse("#ffffff").unwrap());
        style.set_font_family(FontFamily::new("Mono".to_string()));
        style.set_font_size(FontSize::new(14.0));

        let inherited = InheritedStyle::default().descend(&style, Arc::new(HashMap::new()));
        assert_eq!(
            inherited.color(),
            Some(&DrawingColor::parse("#ffffff").unwrap())
        );
        assert_eq!(inherited.font_family().map(AsRef::as_ref), Some("Mono"));
        assert_eq!(inherited.font_size().map(FontSize::value), Some(14.0));
        assert_eq!(inherited.accent_color(), None);
    }

    #[test]
    fn test_root_font_size_carries_forward_unchanged() {
        let root = InheritedStyle::default();
        assert!((root.root_font_size().value() - DEFAULT_FONT_SIZE).abs() < f32::EPSILON);

        let mut style = ComputedStyle::default();
        style.set_font_size(FontSize::new(20.0));
        let child = root.descend(&style, Arc::new(HashMap::new()));
        // The element's own font-size changed; the rem base did not.
        assert!((child.root_font_size().value() - DEFAULT_FONT_SIZE).abs() < f32::EPSILON);
    }

    #[test]
    fn test_custom_properties_carry_forward() {
        let mut props = HashMap::new();
        props.insert("--bg".to_string(), "#1a1b26".to_string());
        let root = InheritedStyle::default().descend(&ComputedStyle::default(), Arc::new(props));

        assert_eq!(
            root.custom_properties().get("--bg").map(String::as_str),
            Some("#1a1b26")
        );
    }
}
