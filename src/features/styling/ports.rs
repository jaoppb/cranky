//! The traits separating the styling domain from CSS parsing, stylesheet loading and reload
//! notification.
use crate::features::styling::domain::{
    ComputedStyle, ElementQuery, InheritedStyle, RuleMatch, StyleSheetName, StylingError,
};
use std::sync::Arc;

pub(crate) trait StyleReloadSender: Send + Sync {
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
    /// Parses `name: value` as a single declaration and returns the style it sets; an invalid
    /// declaration yields an empty style, so it simply has no effect.
    fn reparse(&self, name: &str, value: &str) -> ComputedStyle;
}

/// One stylesheet after parsing, able to report which of its rules match an element and to
/// resolve a style from them alone.
pub trait ParsedStyleSheetPort: Send + Sync {
    /// The sheet's name; the composite resolver puts the sheet named `base` in the lowest cascade
    /// layer.
    fn name(&self) -> &StyleSheetName;
    /// Resolves this sheet's rules for `query`, applying inheritance from
    /// `inherited` and substituting any `var()` references via `reparser`.
    /// Returns the resolved style alongside the context `query`'s children
    /// should inherit from.
    fn resolve_style(
        &self,
        query: &ElementQuery<'_>,
        inherited: &InheritedStyle,
        reparser: &dyn PropertyReparser,
    ) -> (ComputedStyle, InheritedStyle);
    /// Every rule in this sheet that matches `query`, each tagged with its
    /// importance, specificity, and position in this sheet's source order.
    /// Layer and cross-sheet ordering are the composite resolver's job —
    /// this sheet doesn't know about any other.
    fn matching_rules(&self, query: &ElementQuery<'_>) -> Vec<RuleMatch>;
}

/// Turns CSS source into a [`ParsedStyleSheetPort`]; the registry calls it for `base` and each
/// sheet a module declares whenever it builds that module's resolver.
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

pub(crate) trait StyleLoaderPort: Send + Sync {
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

/// Computes an element's style from all of a module's stylesheets together; the vdom bridge
/// calls it for every node while building the styled tree.
pub trait StyleResolverPort: Send + Sync {
    /// Resolves the final style for `query`, applying inheritance from
    /// `inherited`. Returns the resolved style alongside the context
    /// `query`'s children should inherit from.
    fn resolve_style(
        &self,
        query: &ElementQuery<'_>,
        inherited: &InheritedStyle,
    ) -> (ComputedStyle, InheritedStyle);
}
