use crate::features::module_runtime::ports::CommandSender;
use crate::features::styling::domain::{ComputedStyle, ElementQuery, StyleSheetName, StylingError};
use std::sync::Arc;

pub trait ParsedStyleSheetPort: Send + Sync {
    fn name(&self) -> &StyleSheetName;
    fn resolve_style(&self, query: &ElementQuery) -> ComputedStyle;
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
        command_tx: Arc<dyn CommandSender>,
    ) -> Result<Box<dyn notify::Watcher>, StylingError>;
}

pub trait StyleResolverPort: Send + Sync {
    fn resolve_style(&self, query: &ElementQuery) -> ComputedStyle;
}
