//! What one rule's declaration list yields before it becomes a `DeclaredStyle`, grouped by how
//! the cascade treats it: plain values, font-relative lengths, and `var()` data.
use super::super::computed_style::ComputedStyle;
use super::declared_length::DeclaredLength;
use super::declared_lengths::DeclaredLengths;
use super::declared_style::InheritableKeywords;
use std::collections::HashMap;

/// A rule's values as the plain property pipeline parsed them, with the CSS-wide keywords
/// (`inherit`, `initial`) it found on the five inheritable properties.
#[derive(Debug, Clone)]
pub(crate) struct DeclaredValues {
    computed: ComputedStyle,
    keywords: InheritableKeywords,
}

impl DeclaredValues {
    /// The parsed `computed` style and the `keywords` found next to it.
    #[must_use]
    pub(crate) const fn new(computed: ComputedStyle, keywords: InheritableKeywords) -> Self {
        Self { computed, keywords }
    }

    pub(super) fn into_parts(self) -> (ComputedStyle, InheritableKeywords) {
        (self.computed, self.keywords)
    }
}

/// A rule's declarations in font-relative units (`em`, `rem`), which only resolve once the
/// element's font sizes are known: `font-size`'s own and every other length's.
#[derive(Debug, Clone, Default)]
pub(crate) struct FontRelativeLengths {
    font_size: Option<DeclaredLength>,
    lengths: DeclaredLengths,
}

impl FontRelativeLengths {
    /// `font-size`'s font-relative declaration, if any, and the other lengths declared so.
    #[must_use]
    pub(crate) const fn new(font_size: Option<DeclaredLength>, lengths: DeclaredLengths) -> Self {
        Self { font_size, lengths }
    }

    pub(super) const fn into_parts(self) -> (Option<DeclaredLength>, DeclaredLengths) {
        (self.font_size, self.lengths)
    }
}

/// A rule's `var()` data, which only means something against the element's inherited custom
/// properties: its own `--name: value` declarations, and the properties whose value uses
/// `var(...)`.
#[derive(Debug, Clone, Default)]
pub(crate) struct VarDeclarations {
    custom_properties: HashMap<String, String>,
    pending_vars: HashMap<String, String>,
}

impl VarDeclarations {
    /// The rule's `custom_properties` and its `pending_vars`, each keyed by property name.
    #[must_use]
    pub(crate) const fn new(
        custom_properties: HashMap<String, String>,
        pending_vars: HashMap<String, String>,
    ) -> Self {
        Self {
            custom_properties,
            pending_vars,
        }
    }

    pub(super) fn into_parts(self) -> (HashMap<String, String>, HashMap<String, String>) {
        (self.custom_properties, self.pending_vars)
    }
}
