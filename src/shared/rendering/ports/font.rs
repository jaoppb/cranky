//! The port that checks a font family name against the installed fonts.
/// Tells whether a font family name matches an installed font. `ConfigAdapter` passes one to
/// `ConfigDto::into_domain`, which does not call it yet.
pub trait FontValidatorPort: Send + Sync {
    /// Whether `family` names a font family installed on the system, compared case-insensitively;
    /// an empty name counts as valid.
    #[must_use]
    fn is_valid_family(&self, family: &str) -> bool;
}
