pub trait FontValidatorPort: Send + Sync {
    fn is_valid_family(&self, family: &str) -> bool;
}
