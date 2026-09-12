use crate::features::styling::domain::{
    ComputedStyle, ElementQuery, InheritedStyle, RuleMatch, StyleSheetName, StylingError,
};
use std::sync::Arc;

pub trait StyleReloadSender: Send + Sync {
    fn reload_style(&self, name: StyleSheetName);
}

impl<F> StyleReloadSender for F
where
    F: Fn(StyleSheetName) + Send + Sync,
{
    fn reload_style(&self, name: StyleSheetName) {
        self(name);
    }
}

/// Re-parses one CSS declaration after `var()` substitution has replaced
/// any custom-property references with concrete text.
///
/// A declaration containing `var(...)` can't be parsed into a typed value
/// at rule-parse time — the substitution depends on the element's resolved
/// custom properties, which only exist once inheritance has been applied
/// for that specific element. Implemented by whichever adapter owns actual
/// CSS parsing, so the domain cascade can ask for a re-parse without
/// depending on it directly.
pub trait PropertyReparser: Send + Sync {
    fn reparse(&self, name: &str, value: &str) -> ComputedStyle;
}

pub trait ParsedStyleSheetPort: Send + Sync {
    fn name(&self) -> &StyleSheetName;
    /// Resolves this sheet's rules for `query`, applying inheritance from
    /// `inherited` and substituting any `var()` references via `reparser`.
    /// Returns the resolved style alongside the context `query`'s children
    /// should inherit from.
    fn resolve_style(
        &self,
        query: &ElementQuery,
        inherited: &InheritedStyle,
        reparser: &dyn PropertyReparser,
    ) -> (ComputedStyle, InheritedStyle);
    /// Every rule in this sheet that matches `query`, each tagged with its
    /// importance, specificity, and position in this sheet's source order.
    /// Layer and cross-sheet ordering are the composite resolver's job —
    /// this sheet doesn't know about any other.
    fn matching_rules(&self, query: &ElementQuery) -> Vec<RuleMatch>;
}

pub trait CssParserPort: Send + Sync {
    /// Parses a stylesheet from CSS source.
    ///
    /// # Errors
    ///
    /// Returns `StylingError` if CSS parsing fails.
    fn parse_stylesheet(
        &self,
        name: StyleSheetName,
        css_source: &str,
    ) -> Result<Box<dyn ParsedStyleSheetPort>, StylingError>;
}

pub trait StyleLoaderPort: Send + Sync {
    /// Loads a stylesheet from disk.
    ///
    /// # Errors
    ///
    /// Returns `StylingError` if loading fails or the file does not exist.
    fn load_stylesheet(&self, name: &StyleSheetName) -> Result<String, StylingError>;

    /// Ensures built-in stylesheets exist in the style directory.
    ///
    /// # Errors
    ///
    /// Returns `StylingError` if creating default style files fails.
    fn ensure_builtin_styles(&self) -> Result<(), StylingError>;

    /// Sets up a filesystem watcher for stylesheets.
    ///
    /// # Errors
    ///
    /// Returns `StylingError` if watcher initialization fails.
    fn watch_styles(
        &self,
        command_tx: Arc<dyn StyleReloadSender>,
    ) -> Result<Box<dyn notify::Watcher>, StylingError>;
}

pub trait StyleResolverPort: Send + Sync {
    /// Resolves the final style for `query`, applying inheritance from
    /// `inherited`. Returns the resolved style alongside the context
    /// `query`'s children should inherit from.
    fn resolve_style(
        &self,
        query: &ElementQuery,
        inherited: &InheritedStyle,
    ) -> (ComputedStyle, InheritedStyle);
}
