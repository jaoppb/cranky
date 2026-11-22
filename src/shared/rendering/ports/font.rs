//! The port that checks a font family name against the installed fonts, and the default font
//! text is shaped with.
/// Tells whether a font family name matches an installed font. `ConfigAdapter` passes one to
/// `ConfigDto::into_domain`, which does not call it yet.
pub trait FontValidatorPort: Send + Sync {
    /// Whether `family` names a font family installed on the system, compared case-insensitively;
    /// an empty name counts as valid.
    #[must_use]
    fn is_valid_family(&self, family: &str) -> bool;
}

/// The font text is shaped with when its style names none: a family and a size in logical
/// pixels.
#[derive(Debug, Clone, PartialEq)]
pub struct Font {
    family: crate::shared::config::domain::FontFamily,
    size: crate::shared::config::domain::FontSize,
}

impl Font {
    /// Pairs a family with a size.
    #[must_use]
    pub const fn new(
        family: crate::shared::config::domain::FontFamily,
        size: crate::shared::config::domain::FontSize,
    ) -> Self {
        Self { family, size }
    }

    /// The font every render pass falls back to when a node's style names none: the system's
    /// default family at 14 logical pixels, shared so layout, painting and floating content
    /// always measure the same text the same way.
    #[must_use]
    pub(crate) const fn fallback() -> Self {
        Self::new(
            crate::shared::config::domain::FontFamily::new(String::new()),
            crate::shared::config::domain::FontSize::new(14.0),
        )
    }

    #[must_use]
    pub(crate) const fn family(&self) -> &crate::shared::config::domain::FontFamily {
        &self.family
    }

    #[must_use]
    pub(crate) const fn size(&self) -> crate::shared::config::domain::FontSize {
        self.size
    }
}
