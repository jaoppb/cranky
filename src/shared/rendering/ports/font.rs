//! The port that checks a font family name against the installed fonts.
/// Tells whether a font family name matches an installed font, for checking the configured font.
pub trait FontValidatorPort: Send + Sync {
    /// Whether `family` names a font family installed on the system.
    #[must_use]
    fn is_valid_family(&self, family: &str) -> bool;
}
