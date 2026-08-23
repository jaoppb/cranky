use crate::features::module_runtime::ports::CommandSender;
use crate::features::styling::domain::{ComputedStyle, ElementQuery, StyleSheetName, StylingError};
use std::sync::Arc;

pub trait ParsedStyleSheetPort: Send + Sync {
    fn name(&self) -> &StyleSheetName;
    fn resolve_style(&self, query: &ElementQuery) -> ComputedStyle;
}

pub trait CssParserPort: Send + Sync {
    fn parse_stylesheet(
        &self,
        name: StyleSheetName,
        css_source: &str,
    ) -> Result<Box<dyn ParsedStyleSheetPort>, StylingError>;
}

pub trait StyleLoaderPort: Send + Sync {
    fn load_stylesheet(&self, name: &StyleSheetName) -> Result<String, StylingError>;
    fn ensure_builtin_styles(&self) -> Result<(), StylingError>;
    fn watch_styles(
        &self,
        command_tx: Arc<dyn CommandSender>,
    ) -> Result<Box<dyn notify::Watcher>, StylingError>;
}

pub trait StyleResolverPort: Send + Sync {
    fn resolve_style(&self, query: &ElementQuery) -> ComputedStyle;
}
