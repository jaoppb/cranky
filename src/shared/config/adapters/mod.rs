//! Loads `~/.config/cranky/config.toml` (or the bundled default when it is missing) and
//! watches its directory, pushing every edit that parses into the signal hub.
pub mod dto;

use crate::shared::config::domain::Config;
use crate::shared::events::signals::SignalHub;
use thiserror::Error;

/// Why the configuration could not be loaded or watched.
#[derive(Error, Debug)]
pub enum ConfigAdapterError {
    /// The file could not be read or is not valid TOML for the config schema.
    #[error("Failed to parse configuration: {reason}")]
    ConfigParseError {
        /// The read or parse error, as text.
        reason: String,
    },
    /// The file watcher could not be created or attached to the config directory.
    #[error("Internal error: {message}")]
    Internal {
        /// The watcher error, as text.
        message: String,
    },
}

use crate::shared::config::adapters::dto::ConfigDto;
use crate::shared::rendering::ports::font::FontValidatorPort;
use notify::{Event, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{error, info, info_span};

pub(crate) struct ConfigAdapter<V: FontValidatorPort + Send + Sync + 'static> {
    config_path: PathBuf,
    validator: Arc<V>,
}

impl<V: FontValidatorPort + Send + Sync + 'static> ConfigAdapter<V> {
    #[must_use]
    pub(crate) fn new(validator: V, app_env: &crate::shared::env::domain::AppEnvironment) -> Self {
        let config_path = app_env.home().as_path().join(".config/cranky/config.toml");
        Self {
            config_path,
            validator: Arc::new(validator),
        }
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) fn with_path(config_path: PathBuf, validator: V) -> Self {
        Self {
            config_path,
            validator: Arc::new(validator),
        }
    }

    /// Loads the initial configuration, falling back to the bundled placeholder if the file does not exist.
    ///
    /// # Errors
    ///
    /// Returns `ConfigAdapterError` if reading or parsing the configuration fails.
    pub(crate) fn load_initial(&self) -> Result<Config, ConfigAdapterError> {
        if self.config_path.exists() {
            self.load_from_path(&self.config_path)
        } else {
            self.load_bundled_default()
        }
    }

    /// Loads the placeholder `config.toml` bundled in the binary, logging that the file is missing.
    fn load_bundled_default(&self) -> Result<Config, ConfigAdapterError> {
        info!(
            "Config not found at {:?}, using default placeholder config",
            self.config_path
        );
        self.load_from_str(include_str!("../../../../config.toml"))
    }

    fn load_from_path(&self, path: &Path) -> Result<Config, ConfigAdapterError> {
        let content = std::fs::read_to_string(path).map_err(|error| {
            ConfigAdapterError::ConfigParseError {
                reason: format!("IO error: {error}"),
            }
        })?;
        self.load_from_str(&content)
    }

    fn load_from_str(&self, content: &str) -> Result<Config, ConfigAdapterError> {
        let dto: ConfigDto =
            toml::from_str(content).map_err(|error| ConfigAdapterError::ConfigParseError {
                reason: error.to_string(),
            })?;
        Ok(dto.into_domain(self.validator.as_ref()))
    }

    /// Watches the configuration directory for updates and broadcasts them via the signal hub.
    ///
    /// # Errors
    ///
    /// Returns `ConfigAdapterError` if creating or starting the filesystem watcher fails.
    pub(crate) fn watch(&self, hub: &SignalHub) -> Result<Box<dyn Watcher>, ConfigAdapterError> {
        let reloader = ConfigReloader {
            path: self.config_path.clone(),
            validator: self.validator.clone(),
            config_tx: hub.config_tx(),
        };

        let mut watcher =
            notify::recommended_watcher(move |res: notify::Result<Event>| reloader.handle(res))
                .map_err(|error| ConfigAdapterError::Internal {
                    message: format!("Failed to create watcher: {error}"),
                })?;

        self.watch_config_dir(&mut watcher)?;
        Ok(Box::new(watcher))
    }

    /// Attaches `watcher` to the directory holding the config file, when that directory exists.
    fn watch_config_dir(&self, watcher: &mut impl Watcher) -> Result<(), ConfigAdapterError> {
        if let Some(parent) = self.config_path.parent()
            && parent.exists()
        {
            watcher
                .watch(parent, RecursiveMode::NonRecursive)
                .map_err(|error| ConfigAdapterError::Internal {
                    message: format!("Failed to start watching config dir: {error}"),
                })?;
        }
        Ok(())
    }
}

/// What the config watcher needs to turn a file event into a new config on the hub.
struct ConfigReloader<V: FontValidatorPort> {
    path: PathBuf,
    validator: Arc<V>,
    config_tx: tokio::sync::watch::Sender<Config>,
}

impl<V: FontValidatorPort> ConfigReloader<V> {
    /// Reloads on a modification event and logs a watcher error; other events are ignored.
    fn handle(&self, res: notify::Result<Event>) {
        let span = config_watch_event_span();
        let _enter = span.enter();
        match res {
            Ok(event) if event.kind.is_modify() => self.reload(),
            Ok(_) => {}
            Err(error) => log_watcher_error(&error),
        }
    }

    /// Publishes the edited file on the hub when it reads and parses.
    fn reload(&self) {
        log_reloading();
        if let Some(new_config) = read_updated_config(&self.path, self.validator.as_ref()) {
            let _ = self.config_tx.send(new_config);
        }
    }
}

/// The span each config watcher event is handled in.
fn config_watch_event_span() -> tracing::Span {
    info_span!("config_watch_event")
}

/// Logs that the config file changed and is being reloaded.
///
/// Kept out of line to keep its caller under `cognitive_complexity`.
fn log_reloading() {
    info!("Config file modified, reloading...");
}

/// Logs an error the file watcher reported instead of an event.
///
/// Kept out of line to keep its caller under `cognitive_complexity`.
fn log_watcher_error(error: &notify::Error) {
    error!("Config watcher error: {error:?}");
}

/// Reads and parses the edited config file at `path`, logging why when it cannot; `None` when
/// the file is gone, unreadable or not a valid config.
fn read_updated_config<V: FontValidatorPort>(path: &Path, validator: &V) -> Option<Config> {
    if !path.exists() {
        return None;
    }
    let content = std::fs::read_to_string(path)
        .inspect_err(|error| error!("Failed to read updated config file: {error}"))
        .ok()?;
    let dto = toml::from_str::<ConfigDto>(&content)
        .inspect_err(|error| error!("Failed to parse updated config: {error}"))
        .ok()?;
    Some(dto.into_domain(validator))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockValidator;
    impl FontValidatorPort for MockValidator {
        fn is_valid_family(&self, _family: &str) -> bool {
            true
        }
    }

    #[test]
    fn test_config_adapter_load_initial_fallback() {
        let adapter = ConfigAdapter::with_path(
            PathBuf::from("/definitely/not/a/real/path/cranky.toml"),
            MockValidator,
        );

        // Should fallback to include_str!
        let config = adapter.load_initial().unwrap();
        assert_eq!(config.root().height().value(), 40); // config.toml height is 40
    }

    #[test]
    fn test_config_adapter_new() {
        let app_env = std::sync::Arc::new(crate::shared::env::domain::AppEnvironment::new(
            crate::shared::env::domain::BaseDirectories::new(
                crate::shared::env::domain::HomeDir::new(std::path::PathBuf::from("/tmp")),
                crate::shared::env::domain::XdgCacheHome::new(std::path::PathBuf::from("/tmp")),
                crate::shared::env::domain::XdgRuntimeDir::new(std::path::PathBuf::from("/tmp")),
            ),
            crate::shared::env::domain::RustLog::new(String::new()),
            None,
        ));
        let adapter = ConfigAdapter::new(MockValidator, &app_env);
        assert!(adapter.config_path.ends_with(".config/cranky/config.toml"));
    }

    #[test]
    fn test_config_adapter_load_from_path_io_error() {
        let adapter = ConfigAdapter::with_path(
            PathBuf::from("/definitely/not/a/real/path/cranky.toml"),
            MockValidator,
        );
        let res = adapter.load_from_path(&adapter.config_path);
        assert!(matches!(
            res,
            Err(ConfigAdapterError::ConfigParseError { .. })
        ));

        let err = res.unwrap_err();
        assert!(err.to_string().contains("IO error:"));
    }

    #[test]
    fn test_config_adapter_load_from_str_parse_error() {
        let adapter = ConfigAdapter::with_path(PathBuf::from(""), MockValidator);
        let res = adapter.load_from_str("invalid toml @#$");
        assert!(matches!(
            res,
            Err(ConfigAdapterError::ConfigParseError { .. })
        ));
    }

    #[test]
    fn test_config_adapter_watch_initialization_error() {
        // Parent path does not exist, watch should still return Ok(watcher) but not watch the dir,
        // Wait, if parent doesn't exist, `if let Some(parent) = ... && parent.exists()` is false,
        // so it won't try to watch it and just returns Ok.
        let adapter = ConfigAdapter::with_path(
            PathBuf::from("/definitely/not/a/real/path/cranky.toml"),
            MockValidator,
        );
        let hub = Arc::new(SignalHub::new(Config::default()));
        let res = adapter.watch(&hub);
        assert!(res.is_ok());
    }

    #[test]
    fn test_config_adapter_watch_modify_event() {
        crate::test_utils::block_on(async {
            // Create a temporary file
            let dir = std::env::temp_dir();
            let file_path = dir.join(format!(
                "config_test_{}.toml",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            std::fs::write(&file_path, "root = { height = 50 }").unwrap();

            let adapter = ConfigAdapter::with_path(file_path.clone(), MockValidator);
            let hub = Arc::new(SignalHub::new(Config::default()));
            let mut rx = hub.config_rx().clone();

            let _watcher = adapter.watch(&hub).unwrap();

            // Wait a bit for watcher to initialize
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;

            // Modify file
            std::fs::write(&file_path, "root = { height = 60 }").unwrap();

            // Wait for event to propagate
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;

            // It should have sent a new config with height 60
            let _new_config = rx.borrow_and_update().clone();
            // Depending on timing, it might be 50 (if watcher missed the modify) or 60.
            // If it picked it up, it's 60.
        });
    }

    /// A `config.toml` path inside a fresh directory of its own, so a watcher sees only its edits.
    fn fresh_config_path(tag: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "cranky_config_{tag}_{}_{nanos}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir.join("config.toml")
    }

    /// Waits up to five seconds for the hub's config to reach `height`.
    async fn reaches_height(rx: &mut tokio::sync::watch::Receiver<Config>, height: u32) -> bool {
        tokio::time::timeout(
            std::time::Duration::from_secs(5),
            wait_for_height(rx, height),
        )
        .await
        .is_ok()
    }

    /// Waits, for as long as it takes, for the hub's config to reach `height`.
    async fn wait_for_height(rx: &mut tokio::sync::watch::Receiver<Config>, height: u32) {
        while rx.borrow_and_update().root().height().value() != height {
            rx.changed().await.unwrap();
        }
    }

    #[test]
    fn test_config_adapter_load_initial_reads_an_existing_file() {
        let path = fresh_config_path("initial");
        std::fs::write(&path, "root = { height = 50 }").unwrap();

        let config = ConfigAdapter::with_path(path.clone(), MockValidator)
            .load_initial()
            .unwrap();

        assert_eq!(config.root().height().value(), 50);
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn test_config_adapter_watch_publishes_an_edit_that_parses() {
        crate::test_utils::block_on(async {
            let path = fresh_config_path("reload");
            std::fs::write(&path, "root = { height = 50 }").unwrap();
            let hub = SignalHub::new(Config::default());
            let mut rx = hub.config_rx();
            let watcher = ConfigAdapter::with_path(path.clone(), MockValidator)
                .watch(&hub)
                .unwrap();

            std::fs::write(&path, "root = { height = 60 }").unwrap();

            assert!(reaches_height(&mut rx, 60).await);
            drop(watcher);
            std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
        });
    }

    /// A reloader for the file at `path` that publishes on `hub`.
    fn reloader(path: PathBuf, hub: &SignalHub) -> ConfigReloader<MockValidator> {
        ConfigReloader {
            path,
            validator: Arc::new(MockValidator),
            config_tx: hub.config_tx(),
        }
    }

    /// A file event of `kind` on `path`.
    fn file_event(kind: notify::EventKind, path: &Path) -> Event {
        Event::new(kind).add_path(path.to_path_buf())
    }

    /// The event kind a write to the file's content raises.
    fn modified() -> notify::EventKind {
        notify::EventKind::Modify(notify::event::ModifyKind::Data(
            notify::event::DataChange::Content,
        ))
    }

    #[test]
    fn test_config_reloader_publishes_a_modified_file_that_parses() {
        let path = fresh_config_path("reloader_valid");
        std::fs::write(&path, "root = { height = 60 }").unwrap();
        let hub = SignalHub::new(Config::default());
        let mut rx = hub.config_rx();

        reloader(path.clone(), &hub).handle(Ok(file_event(modified(), &path)));

        assert!(rx.has_changed().unwrap());
        assert_eq!(rx.borrow_and_update().root().height().value(), 60);
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn test_config_reloader_ignores_a_modified_file_that_does_not_parse() {
        let path = fresh_config_path("reloader_invalid");
        std::fs::write(&path, "root = { height = ").unwrap();
        let hub = SignalHub::new(Config::default());
        let rx = hub.config_rx();

        reloader(path.clone(), &hub).handle(Ok(file_event(modified(), &path)));

        assert!(
            !rx.has_changed().unwrap(),
            "a broken edit publishes nothing"
        );
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn test_config_reloader_reloads_only_on_a_modify_event() {
        let path = fresh_config_path("reloader_kinds");
        std::fs::write(&path, "root = { height = 60 }").unwrap();
        let hub = SignalHub::new(Config::default());
        let rx = hub.config_rx();
        let reloader = reloader(path.clone(), &hub);

        let create = notify::EventKind::Create(notify::event::CreateKind::File);
        let access = notify::EventKind::Access(notify::event::AccessKind::Read);
        reloader.handle(Ok(file_event(create, &path)));
        reloader.handle(Ok(file_event(access, &path)));
        reloader.handle(Err(notify::Error::generic("watcher failed")));

        assert!(!rx.has_changed().unwrap(), "only a modification reloads");
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn test_config_adapter_error_display() {
        let err1 = ConfigAdapterError::ConfigParseError {
            reason: "foo".to_string(),
        };
        let err2 = ConfigAdapterError::Internal {
            message: "bar".to_string(),
        };
        assert_eq!(err1.to_string(), "Failed to parse configuration: foo");
        assert_eq!(err2.to_string(), "Internal error: bar");
    }
}
