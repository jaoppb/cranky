use crate::features::module_runtime::ports::CommandSender;
use crate::features::styling::domain::{ComputedStyle, ElementQuery, StyleSheetName, StylingError};
use crate::features::styling::ports::{ParsedStyleSheetPort, StyleLoaderPort, StyleResolverPort};
use crate::shared::env::domain::AppEnvironment;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

pub struct CompositeStyleResolver {
    stylesheets: Vec<Box<dyn ParsedStyleSheetPort>>,
}

impl CompositeStyleResolver {
    pub fn new(stylesheets: Vec<Box<dyn ParsedStyleSheetPort>>) -> Self {
        Self { stylesheets }
    }
}

impl StyleResolverPort for CompositeStyleResolver {
    fn resolve_style(&self, query: &ElementQuery) -> ComputedStyle {
        let mut computed = ComputedStyle::default();
        for sheet in &self.stylesheets {
            let s = sheet.resolve_style(query);
            computed.merge_with(&s);
        }
        computed
    }
}

pub struct FsStyleLoader {
    app_env: Arc<AppEnvironment>,
}

impl FsStyleLoader {
    pub const BUILTIN_STYLES: &[(&'static str, &'static str)] = &[
        ("base", include_str!("../../../../assets/styles/base.css")),
        ("bar", include_str!("../../../../assets/styles/bar.css")),
        ("hour", include_str!("../../../../assets/styles/hour.css")),
        (
            "workspace",
            include_str!("../../../../assets/styles/workspace.css"),
        ),
        (
            "metrics",
            include_str!("../../../../assets/styles/metrics.css"),
        ),
        (
            "systray",
            include_str!("../../../../assets/styles/systray.css"),
        ),
        ("mpris", include_str!("../../../../assets/styles/mpris.css")),
    ];

    pub fn new(app_env: Arc<AppEnvironment>) -> Self {
        Self { app_env }
    }

    pub fn builtin_content(name: &str) -> Option<&'static str> {
        Self::BUILTIN_STYLES
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, c)| *c)
    }
}

impl StyleLoaderPort for FsStyleLoader {
    fn ensure_builtin_styles(&self) -> Result<(), StylingError> {
        let home = self.app_env.home().as_path();
        let dir = PathBuf::from(home).join(".local/share/cranky/styles");

        tracing::debug!(dir = ?dir, "Ensuring built-in stylesheets are present on disk");
        fs::create_dir_all(&dir).map_err(|e| StylingError::LoaderError(e.to_string()))?;

        for (name, content) in Self::BUILTIN_STYLES {
            let path = dir.join(format!("{}.css", name));
            if !path.exists() || fs::read_to_string(&path).ok().as_deref() != Some(*content) {
                tracing::debug!(path = ?path, stylesheet = %name, "Writing built-in stylesheet to disk");
                let _ = fs::write(&path, content);
            }
        }

        Ok(())
    }

    fn load_stylesheet(&self, name: &StyleSheetName) -> Result<String, StylingError> {
        let home = self.app_env.home().as_path();
        let user_dir = PathBuf::from(home).join(".config/cranky/styles");
        let shadow_dir = PathBuf::from(home).join(".local/share/cranky/styles");

        let file_name = format!("{}.css", name.as_str());

        // 1. Check user config directory
        let user_file = user_dir.join(&file_name);
        if let Ok(content) = fs::read_to_string(&user_file) {
            tracing::debug!(stylesheet = %name.as_str(), source = "user_config", path = ?user_file, "Loaded stylesheet");
            return Ok(content);
        }

        // 2. Check local share directory
        let shadow_file = shadow_dir.join(&file_name);
        if let Ok(content) = fs::read_to_string(&shadow_file) {
            tracing::debug!(stylesheet = %name.as_str(), source = "local_share", path = ?shadow_file, "Loaded stylesheet");
            return Ok(content);
        }

        // 3. Fallback to embedded builtin
        if let Some(content) = Self::builtin_content(name.as_str()) {
            tracing::debug!(stylesheet = %name.as_str(), source = "embedded_builtin", "Loaded stylesheet");
            return Ok(content.to_string());
        }

        tracing::warn!(stylesheet = %name.as_str(), "Stylesheet not found in any search path");
        Err(StylingError::LoaderError(format!(
            "Stylesheet '{}' not found",
            name.as_str()
        )))
    }

    fn watch_styles(
        &self,
        command_tx: Arc<dyn CommandSender>,
    ) -> Result<Box<dyn notify::Watcher>, StylingError> {
        use notify::{Event, RecursiveMode, Watcher};

        let home = self.app_env.home().as_path();
        let user_dir = PathBuf::from(home).join(".config/cranky/styles");
        let shadow_dir = PathBuf::from(home).join(".local/share/cranky/styles");

        let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
            if let Ok(event) = res
                && event.kind.is_modify()
            {
                for path in event.paths {
                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                        tracing::debug!("Stylesheet modified: {:?}", path);
                        if let Ok(sheet_name) = StyleSheetName::new(stem) {
                            command_tx.send_command(crate::app::commands::AppCommand::ReloadStyle(
                                sheet_name,
                            ));
                        }
                    }
                }
            }
        })
        .map_err(|e| StylingError::LoaderError(format!("Failed to create style watcher: {}", e)))?;

        if user_dir.exists() {
            let _ = watcher.watch(&user_dir, RecursiveMode::NonRecursive);
        }
        if shadow_dir.exists() {
            let _ = watcher.watch(&shadow_dir, RecursiveMode::NonRecursive);
        }

        Ok(Box::new(watcher))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_test_env(sub: &str) -> Arc<AppEnvironment> {
        let dir = std::env::temp_dir().join(format!("cranky_test_styling_{}", sub));
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::create_dir_all(&dir);
        Arc::new(AppEnvironment::new(
            crate::shared::env::domain::HomeDir::new(dir.clone()),
            crate::shared::env::domain::XdgCacheHome::new(dir.clone()),
            crate::shared::env::domain::XdgRuntimeDir::new(dir),
            crate::shared::env::domain::RustLog::new(String::new()),
            None,
        ))
    }

    #[test]
    fn test_ensure_builtin_styles() {
        let env = get_test_env("ensure");
        let loader = FsStyleLoader::new(env);
        assert!(loader.ensure_builtin_styles().is_ok());
    }

    #[test]
    fn test_load_stylesheet_fallback() {
        let env = get_test_env("fallback");
        let loader = FsStyleLoader::new(env);
        let base = loader
            .load_stylesheet(&StyleSheetName::new("base").unwrap())
            .unwrap();
        assert!(base.contains("bar {"));
    }
}
