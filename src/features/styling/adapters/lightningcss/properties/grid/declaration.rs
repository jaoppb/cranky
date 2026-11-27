//! Splits a serialized declaration into its property name and value, for the grid appliers
//! that match properties by name rather than by typed variant.

/// The trimmed name and value of a `name: value` declaration, or an empty name and the whole
/// text when it has no colon.
#[must_use]
pub(super) fn split_declaration(full: &str) -> (&str, &str) {
    full.split_once(':')
        .map_or(("", full), |(property, value)| {
            (property.trim(), value.trim())
        })
}
