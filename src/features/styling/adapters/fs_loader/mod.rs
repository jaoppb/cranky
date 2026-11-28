//! Loads, watches and composes the stylesheets modules declare.
//!
//! A sheet comes from `~/.config/cranky/styles/` (user overrides), then the deployed builtins in
//! `~/.local/share/cranky/styles/`, then the embedded copies; both directories are watched for hot
//! reload, and a module's sheets are composed into one resolver.

pub mod composite;
mod watch;

use crate::features::styling::domain::{StyleSheetName, StylingError};
use crate::features::styling::ports::{StyleLoaderPort, StyleReloadSender};
use crate::shared::env::domain::AppEnvironment;
use notify::Event;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use watch::{reload_modified_sheets, watch_if_present};

pub(crate) use composite::CompositeStyleResolver;

pub(crate) struct FsStyleLoader {
    app_env: Arc<AppEnvironment>,
}

impl FsStyleLoader {
    pub(crate) const BUILTIN_STYLES: &[(&'static str, &'static str)] = &[
        (
            "base",
            include_str!("../../../../../assets/styles/base.css"),
        ),
        ("bar", include_str!("../../../../../assets/styles/bar.css")),
        (
            "clock",
            include_str!("../../../../../assets/styles/clock.css"),
        ),
        (
            "calendar",
            include_str!("../../../../../assets/styles/calendar.css"),
        ),
        (
            "workspace",
            include_str!("../../../../../assets/styles/workspace.css"),
        ),
        (
            "metrics",
            include_str!("../../../../../assets/styles/metrics.css"),
        ),
        (
            "systray",
            include_str!("../../../../../assets/styles/systray.css"),
        ),
        (
            "mpris",
            include_str!("../../../../../assets/styles/mpris.css"),
        ),
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

    /// The directory of the user's overriding sheets, `~/.config/cranky/styles`.
    fn user_styles_dir(&self) -> PathBuf {
        PathBuf::from(self.app_env.home().as_path()).join(".config/cranky/styles")
    }

    /// The directory the built-in sheets are deployed to, `~/.local/share/cranky/styles`.
    fn deployed_styles_dir(&self) -> PathBuf {
        PathBuf::from(self.app_env.home().as_path()).join(".local/share/cranky/styles")
    }

    /// Reads sheet `name` from the user directory, else from the deployed one, logging where
    /// it was found.
    fn read_from_disk(&self, name: &StyleSheetName) -> Option<String> {
        let file_name = format!("{name}.css");
        let search_path = [
            ("user_config", self.user_styles_dir()),
            ("local_share", self.deployed_styles_dir()),
        ];
        search_path.into_iter().find_map(|(source, dir)| {
            let path = dir.join(&file_name);
            let content = fs::read_to_string(&path).ok()?;
            log_loaded_from_disk(name, source, &path);
            Some(content)
        })
    }
}

impl StyleLoaderPort for FsStyleLoader {
    fn ensure_builtin_styles(&self) -> Result<(), StylingError> {
        let dir = self.deployed_styles_dir();
        log_ensuring_builtins(&dir);
        fs::create_dir_all(&dir).map_err(|error| StylingError::LoaderError(error.to_string()))?;
        for (name, content) in Self::BUILTIN_STYLES {
            deploy_builtin(&dir, name, content);
        }
        Ok(())
    }

    fn load_stylesheet(&self, name: &StyleSheetName) -> Result<String, StylingError> {
        if let Some(content) = self.read_from_disk(name) {
            return Ok(content);
        }
        if let Some(content) = Self::builtin_content(name.as_str()) {
            log_loaded_embedded(name);
            return Ok(content.to_string());
        }
        log_not_found(name);
        Err(StylingError::LoaderError(format!(
            "Stylesheet '{name}' not found"
        )))
    }

    fn watch_styles(
        &self,
        command_tx: Arc<dyn StyleReloadSender>,
    ) -> Result<Box<dyn notify::Watcher>, StylingError> {
        let user_dir = self.user_styles_dir();
        let deployed_dir = self.deployed_styles_dir();
        let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
            reload_modified_sheets(res, command_tx.as_ref());
        })
        .map_err(|error| {
            StylingError::LoaderError(format!("Failed to create style watcher: {error}"))
        })?;
        watch_if_present(&mut watcher, &user_dir);
        watch_if_present(&mut watcher, &deployed_dir);
        Ok(Box::new(watcher))
    }
}

/// Writes the embedded `content` of sheet `name` into `dir` unless an identical copy is there;
/// a failed write is ignored.
fn deploy_builtin(dir: &Path, name: &str, content: &str) {
    let path = dir.join(format!("{name}.css"));
    if !path.exists() || fs::read_to_string(&path).ok().as_deref() != Some(content) {
        log_writing_builtin(&path, name);
        let _ = fs::write(&path, content);
    }
}

/// Logs that the built-in sheets are about to be deployed into `dir`.
///
/// Kept out of line to keep its caller under `cognitive_complexity`.
fn log_ensuring_builtins(dir: &Path) {
    tracing::debug!(dir = ?dir, "Ensuring built-in stylesheets are present on disk");
}

/// Logs that the built-in sheet `name` is being written to `path`.
///
/// Kept out of line to keep its caller under `cognitive_complexity`.
fn log_writing_builtin(path: &Path, name: &str) {
    tracing::debug!(path = ?path, stylesheet = %name, "Writing built-in stylesheet to disk");
}

/// Logs that sheet `name` was loaded from `path`, found in the directory `source` names.
///
/// Kept out of line to keep its caller under `cognitive_complexity`.
fn log_loaded_from_disk(name: &StyleSheetName, source: &str, path: &Path) {
    tracing::debug!(stylesheet = %name.as_str(), source = source, path = ?path, "Loaded stylesheet");
}

/// Logs that sheet `name` fell back to its embedded copy.
///
/// Kept out of line to keep its caller under `cognitive_complexity`.
fn log_loaded_embedded(name: &StyleSheetName) {
    tracing::debug!(stylesheet = %name.as_str(), source = "embedded_builtin", "Loaded stylesheet");
}

/// Logs that sheet `name` exists in no search path.
///
/// Kept out of line to keep its caller under `cognitive_complexity`.
fn log_not_found(name: &StyleSheetName) {
    tracing::warn!(stylesheet = %name.as_str(), "Stylesheet not found in any search path");
}

/// Logs that the sheet at `path` changed on disk.
///
/// Kept out of line to keep its caller under `cognitive_complexity`.
fn log_sheet_modified(path: &Path) {
    tracing::debug!("Stylesheet modified: {:?}", path);
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
    fn watching_ignores_removals_creations_and_names_that_are_not_sheet_names() {
        let home = TestHome::new("styling");
        let user = home.home().as_path().join(USER_DIR);
        write_style(&user, "gone", "/* gone */");
        let (_watcher, rx) = watch(&home);

        fs::remove_file(user.join("gone.css")).unwrap();
        // Created but never written: a create event, no modification.
        fs::File::create(user.join("touched.css")).unwrap();
        write_style(&user, "not.a.sheet", "/* odd */");
        write_style(&user, "kept", "/* kept */");

        let names = reloads(&rx, std::time::Duration::from_secs(2));
        assert!(names.iter().all(|name| name == "kept"), "{names:?}");
        assert_ne!(names, Vec::<String>::new());
    }

    #[test]
    fn watching_starts_even_when_no_style_directory_exists() {
        let home = TestHome::new("styling");
        // `watch` fails the test when `watch_styles` returns an error.
        let _started = watch(&home);
    }
}
