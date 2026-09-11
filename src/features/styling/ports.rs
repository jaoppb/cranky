use crate::features::styling::domain::{
    ComputedStyle, ElementQuery, RuleMatch, StyleSheetName, StylingError,
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

pub trait ParsedStyleSheetPort: Send + Sync {
    fn name(&self) -> &StyleSheetName;
    fn resolve_style(&self, query: &ElementQuery) -> ComputedStyle;
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
    fn resolve_style(&self, query: &ElementQuery) -> ComputedStyle;
}
