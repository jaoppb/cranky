pub trait FontValidatorPort: Send + Sync {
    #[must_use]
    fn is_valid_family(&self, family: &str) -> bool;
}
