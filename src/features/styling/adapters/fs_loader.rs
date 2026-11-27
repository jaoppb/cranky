//! Loads, watches and composes the stylesheets modules declare.
//!
//! A sheet comes from `~/.config/cranky/styles/` (user overrides), then the deployed builtins in
//! `~/.local/share/cranky/styles/`, then the embedded copies; both directories are watched for hot
//! reload, and a module's sheets are composed into one resolver.

use super::lightningcss::LightningPropertyReparser;
use crate::features::styling::domain::{
    CascadeOrigin, ComputedStyle, ElementQuery, InheritedStyle, Layer, StyleSheetName,
    StylingError, fold_matches,
};
use crate::features::styling::ports::{
    ParsedStyleSheetPort, StyleLoaderPort, StyleReloadSender, StyleResolverPort,
};
use crate::shared::env::domain::AppEnvironment;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

pub(crate) struct CompositeStyleResolver {
    stylesheets: Vec<Box<dyn ParsedStyleSheetPort>>,
}

impl CompositeStyleResolver {
    #[must_use]
    pub(crate) fn new(stylesheets: Vec<Box<dyn ParsedStyleSheetPort>>) -> Self {
        Self { stylesheets }
    }
}

impl StyleResolverPort for CompositeStyleResolver {
    fn resolve_style(
        &self,
        query: &ElementQuery<'_>,
        inherited: &InheritedStyle,
    ) -> (ComputedStyle, InheritedStyle) {
        // "base" is the one shared origin sheet, always loaded first by the
        // registry; every other composed sheet is the module's own. Cascade
        // layer order beats specificity, so base stays overridable by
        // construction rather than by (previously accidental) load order.
        let matches = self
            .stylesheets
            .iter()
            .enumerate()
            .flat_map(|(sheet_index, sheet)| {
                let layer = if sheet.name().as_str() == "base" {
                    Layer::BASE
                } else {
                    Layer::MODULE
                };
                let sheet_index = u32::try_from(sheet_index).unwrap_or(u32::MAX);
                sheet
                    .matching_rules(query)
                    .into_iter()
                    .map(move |rule_match| {
                        rule_match.into_matched(CascadeOrigin::new(layer, sheet_index))
                    })
            })
            .collect();
        fold_matches(matches).collapse(inherited, &LightningPropertyReparser)
    }
}

pub(crate) struct FsStyleLoader {
    app_env: Arc<AppEnvironment>,
}

impl FsStyleLoader {
    pub(crate) const BUILTIN_STYLES: &[(&'static str, &'static str)] = &[
        ("base", include_str!("../../../../assets/styles/base.css")),
        ("bar", include_str!("../../../../assets/styles/bar.css")),
        ("clock", include_str!("../../../../assets/styles/clock.css")),
        (
            "calendar",
            include_str!("../../../../assets/styles/calendar.css"),
        ),
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

    #[must_use]
    pub(crate) const fn new(app_env: Arc<AppEnvironment>) -> Self {
        Self { app_env }
    }

    #[must_use]
    pub(crate) fn builtin_content(name: &str) -> Option<&'static str> {
        Self::BUILTIN_STYLES
            .iter()
            .find(|(style_name, _)| *style_name == name)
            .map(|(_, content)| *content)
    }
}

impl StyleLoaderPort for FsStyleLoader {
    fn ensure_builtin_styles(&self) -> Result<(), StylingError> {
        let home = self.app_env.home().as_path();
        let dir = PathBuf::from(home).join(".local/share/cranky/styles");

        tracing::debug!(dir = ?dir, "Ensuring built-in stylesheets are present on disk");
        fs::create_dir_all(&dir).map_err(|error| StylingError::LoaderError(error.to_string()))?;

        for (name, content) in Self::BUILTIN_STYLES {
            let path = dir.join(format!("{name}.css"));
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

        let file_name = format!("{name}.css");

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
            "Stylesheet '{name}' not found"
        )))
    }

    fn watch_styles(
        &self,
        command_tx: Arc<dyn StyleReloadSender>,
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
                    if let Some(stem) = path.file_stem().and_then(|file_stem| file_stem.to_str()) {
                        tracing::debug!("Stylesheet modified: {:?}", path);
                        if let Ok(sheet_name) = StyleSheetName::new(stem) {
                            command_tx.reload_style(sheet_name);
                        }
                    }
                }
            }
        })
        .map_err(|error| {
            StylingError::LoaderError(format!("Failed to create style watcher: {error}"))
        })?;

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

    use crate::test_utils::TestHome;

    #[test]
    fn test_ensure_builtin_styles() {
        let home = TestHome::new("styling");
        let loader = FsStyleLoader::new(home.env());
        assert!(loader.ensure_builtin_styles().is_ok());
    }

    #[test]
    fn test_load_stylesheet_fallback() {
        let home = TestHome::new("styling");
        let loader = FsStyleLoader::new(home.env());
        let base = loader
            .load_stylesheet(&StyleSheetName::new("base").unwrap())
            .unwrap();
        assert!(base.contains("bar {"));
    }

    fn write_style(dir: &std::path::Path, name: &str, content: &str) {
        fs::create_dir_all(dir).unwrap();
        fs::write(dir.join(format!("{name}.css")), content).unwrap();
    }

    fn load(home: &TestHome, name: &str) -> Result<String, StylingError> {
        FsStyleLoader::new(home.env()).load_stylesheet(&StyleSheetName::new(name)?)
    }

    const USER_DIR: &str = ".config/cranky/styles";
    const SHARE_DIR: &str = ".local/share/cranky/styles";

    #[test]
    fn user_stylesheet_wins_over_the_deployed_and_embedded_ones() {
        let home = TestHome::new("styling");
        write_style(
            &home.home().as_path().join(SHARE_DIR),
            "base",
            "/* share */",
        );
        assert_eq!(load(&home, "base").unwrap(), "/* share */");

        write_style(&home.home().as_path().join(USER_DIR), "base", "/* user */");
        assert_eq!(load(&home, "base").unwrap(), "/* user */");
    }

    #[test]
    fn stylesheet_found_nowhere_is_reported_missing_by_name() {
        let home = TestHome::new("styling");
        assert!(matches!(
            load(&home, "mine"),
            Err(StylingError::LoaderError(message)) if message.contains("mine")
        ));
    }

    #[test]
    fn stylesheet_with_no_builtin_is_loaded_from_disk() {
        let home = TestHome::new("styling");
        write_style(&home.home().as_path().join(USER_DIR), "mine", "/* mine */");
        assert_eq!(load(&home, "mine").unwrap(), "/* mine */");
    }

    #[test]
    fn ensure_builtin_styles_deploys_every_builtin_and_repairs_edited_copies() {
        let home = TestHome::new("styling");
        // A stale deployed copy from an older version must be overwritten.
        write_style(
            &home.home().as_path().join(SHARE_DIR),
            "clock",
            "/* stale */",
        );

        FsStyleLoader::new(home.env())
            .ensure_builtin_styles()
            .unwrap();

        let deployed = home.home().as_path().join(SHARE_DIR);
        for (name, content) in FsStyleLoader::BUILTIN_STYLES {
            let on_disk = fs::read_to_string(deployed.join(format!("{name}.css"))).unwrap();
            assert_eq!(on_disk, *content, "{name}");
        }
    }

    #[test]
    fn ensure_builtin_styles_leaves_up_to_date_copies_untouched() {
        let home = TestHome::new("styling");
        let loader = FsStyleLoader::new(home.env());
        loader.ensure_builtin_styles().unwrap();
        let base = home.home().as_path().join(SHARE_DIR).join("base.css");
        let old = std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1000);
        fs::File::options()
            .write(true)
            .open(&base)
            .unwrap()
            .set_modified(old)
            .unwrap();

        loader.ensure_builtin_styles().unwrap();

        assert_eq!(fs::metadata(&base).unwrap().modified().unwrap(), old);
    }

    #[test]
    fn ensure_builtin_styles_skips_a_copy_it_cannot_write_and_deploys_the_rest() {
        let home = TestHome::new("styling");
        let deployed = home.home().as_path().join(SHARE_DIR);
        fs::create_dir_all(deployed.join("base.css")).unwrap();

        assert!(
            FsStyleLoader::new(home.env())
                .ensure_builtin_styles()
                .is_ok()
        );

        assert!(deployed.join("base.css").is_dir());
        let mpris = fs::read_to_string(deployed.join("mpris.css")).unwrap();
        assert_eq!(
            Some(mpris.as_str()),
            FsStyleLoader::builtin_content("mpris")
        );
    }

    #[test]
    fn ensure_builtin_styles_reports_a_directory_it_cannot_create() {
        let home = TestHome::new("styling");
        let share = home.home().as_path().join(".local/share");
        fs::create_dir_all(&share).unwrap();
        fs::write(share.join("cranky"), "not a directory").unwrap();

        assert!(matches!(
            FsStyleLoader::new(home.env()).ensure_builtin_styles(),
            Err(StylingError::LoaderError(_))
        ));
    }

    #[test]
    fn deployed_stylesheet_wins_over_the_embedded_one() {
        let home = TestHome::new("styling");
        write_style(&home.home().as_path().join(SHARE_DIR), "bar", "/* share */");
        assert_eq!(load(&home, "bar").unwrap(), "/* share */");
        assert_eq!(
            load(&home, "clock").unwrap(),
            FsStyleLoader::builtin_content("clock").unwrap()
        );
    }

    /// Starts watching `home`'s style directories, returning the watcher and the sheet names
    /// it asks to reload.
    fn watch(
        home: &TestHome,
    ) -> (
        Box<dyn notify::Watcher>,
        std::sync::mpsc::Receiver<StyleSheetName>,
    ) {
        let (tx, rx) = std::sync::mpsc::channel();
        let sender = move |name: StyleSheetName| {
            let _ = tx.send(name);
        };
        let watcher = FsStyleLoader::new(home.env())
            .watch_styles(Arc::new(sender))
            .unwrap();
        (watcher, rx)
    }

    /// Every sheet name received until none arrives for `wait`, in order.
    fn reloads(
        rx: &std::sync::mpsc::Receiver<StyleSheetName>,
        wait: std::time::Duration,
    ) -> Vec<String> {
        let mut names = Vec::new();
        while let Ok(name) = rx.recv_timeout(wait) {
            names.push(name.as_str().to_owned());
        }
        names
    }

    #[test]
    fn watching_reloads_a_sheet_modified_in_either_style_directory() {
        let home = TestHome::new("styling");
        let user = home.home().as_path().join(USER_DIR);
        let share = home.home().as_path().join(SHARE_DIR);
        fs::create_dir_all(&user).unwrap();
        fs::create_dir_all(&share).unwrap();
        let (_watcher, rx) = watch(&home);

        write_style(&user, "clock", "/* user */");
        write_style(&share, "metrics", "/* share */");

        let names = reloads(&rx, std::time::Duration::from_secs(2));
        assert!(names.contains(&"clock".to_owned()), "{names:?}");
        assert!(names.contains(&"metrics".to_owned()), "{names:?}");
    }

    #[test]
    fn watching_ignores_removals_and_names_that_are_not_sheet_names() {
        let home = TestHome::new("styling");
        let user = home.home().as_path().join(USER_DIR);
        write_style(&user, "gone", "/* gone */");
        let (_watcher, rx) = watch(&home);

        fs::remove_file(user.join("gone.css")).unwrap();
        write_style(&user, "not.a.sheet", "/* odd */");
        write_style(&user, "kept", "/* kept */");

        let names = reloads(&rx, std::time::Duration::from_secs(2));
        assert!(names.iter().all(|name| name == "kept"), "{names:?}");
        assert_ne!(names, Vec::<String>::new());
    }

    #[test]
    fn watching_starts_even_when_no_style_directory_exists() {
        let home = TestHome::new("styling");
        let (_watcher, rx) = watch(&home);
        assert_eq!(
            reloads(&rx, std::time::Duration::from_millis(100)),
            Vec::<String>::new()
        );
    }
}
