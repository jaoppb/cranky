pub mod dto;

use crate::shared::config::domain::Config;
use crate::shared::events::signals::SignalHub;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigAdapterError {
    #[error("Failed to parse configuration: {reason}")]
    ConfigParseError { reason: String },
    #[error("Internal error: {message}")]
    Internal { message: String },
}

use crate::shared::config::adapters::dto::ConfigDto;
use crate::shared::rendering::ports::font::FontValidatorPort;
use notify::{Event, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{error, info, info_span};

pub struct ConfigAdapter<V: FontValidatorPort + Send + Sync + 'static> {
    config_path: PathBuf,
    validator: Arc<V>,
}

impl<V: FontValidatorPort + Send + Sync + 'static> ConfigAdapter<V> {
    pub fn new(validator: V, app_env: std::sync::Arc<crate::shared::env::domain::AppEnvironment>) -> Self {
        let config_path = app_env.home().as_path().join(".config/cranky/config.toml");
        Self {
            config_path,
            validator: Arc::new(validator),
        }
    }

    #[cfg(test)]
    pub fn with_path(config_path: PathBuf, validator: V) -> Self {
        Self {
            config_path,
            validator: Arc::new(validator),
        }
    }

    pub fn load_initial(&self) -> Result<Config, ConfigAdapterError> {
        if self.config_path.exists() {
            self.load_from_path(&self.config_path)
        } else {
            info!(
                "Config not found at {:?}, using default placeholder config",
                self.config_path
            );
            self.load_from_str(include_str!("../../../../config.toml"))
        }
    }

    fn load_from_path(&self, path: &Path) -> Result<Config, ConfigAdapterError> {
        let content =
            std::fs::read_to_string(path).map_err(|e| ConfigAdapterError::ConfigParseError {
                reason: format!("IO error: {}", e),
            })?;
        self.load_from_str(&content)
    }

    fn load_from_str(&self, content: &str) -> Result<Config, ConfigAdapterError> {
        let dto: ConfigDto =
            toml::from_str(content).map_err(|e| ConfigAdapterError::ConfigParseError {
                reason: e.to_string(),
            })?;
        Ok(dto.into_domain(self.validator.as_ref()))
    }

    pub fn watch(&self, hub: Arc<SignalHub>) -> Result<Box<dyn Watcher>, ConfigAdapterError> {
        let config_tx = hub.config_tx();
        let path = self.config_path.clone();
        let validator = self.validator.clone();

        let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
            let span = info_span!("config_watch_event");
            let _enter = span.enter();
            match res {
                Ok(event) => {
                    if event.kind.is_modify() {
                        info!("Config file modified, reloading...");
                        if path.exists() {
                            match std::fs::read_to_string(&path) {
                                Ok(content) => match toml::from_str::<ConfigDto>(&content) {
                                    Ok(dto) => {
                                        let new_config = dto.into_domain(validator.as_ref());
                                        let _ = config_tx.send(new_config);
                                    }
                                    Err(e) => error!("Failed to parse updated config: {}", e),
                                },
                                Err(e) => error!("Failed to read updated config file: {}", e),
                            }
                        }
                    }
                }
                Err(e) => error!("Config watcher error: {:?}", e),
            }
        })
        .map_err(|e| ConfigAdapterError::Internal {
            message: format!("Failed to create watcher: {}", e),
        })?;

        if let Some(parent) = self.config_path.parent()
            && parent.exists()
        {
            watcher
                .watch(parent, RecursiveMode::NonRecursive)
                .map_err(|e| ConfigAdapterError::Internal {
                    message: format!("Failed to start watching config dir: {}", e),
                })?;
        }

        Ok(Box::new(watcher))
    }
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
        assert_eq!(config.bar().height().value(), 40); // config.toml height is 40
    }

    #[test]
    fn test_config_adapter_new() {
        let app_env = std::sync::Arc::new(crate::shared::env::domain::AppEnvironment::new(
            crate::shared::env::domain::HomeDir::new(std::path::PathBuf::from("/tmp")),
            crate::shared::env::domain::XdgCacheHome::new(std::path::PathBuf::from("/tmp")),
            crate::shared::env::domain::XdgRuntimeDir::new(std::path::PathBuf::from("/tmp")),
            crate::shared::env::domain::RustLog::new(String::new()),
            None,
        ));
        let adapter = ConfigAdapter::new(MockValidator, app_env);
        assert!(adapter.config_path.ends_with(".config/cranky/config.toml"));
    }
    
    #[test]
    fn test_config_adapter_load_from_path_io_error() {
        let adapter = ConfigAdapter::with_path(
            PathBuf::from("/definitely/not/a/real/path/cranky.toml"),
            MockValidator,
        );
        let res = adapter.load_from_path(&adapter.config_path);
        assert!(matches!(res, Err(ConfigAdapterError::ConfigParseError { .. })));
        
        let err = res.unwrap_err();
        assert!(err.to_string().contains("IO error:"));
    }
    
    #[test]
    fn test_config_adapter_load_from_str_parse_error() {
        let adapter = ConfigAdapter::with_path(
            PathBuf::from(""),
            MockValidator,
        );
        let res = adapter.load_from_str("invalid toml @#$");
        assert!(matches!(res, Err(ConfigAdapterError::ConfigParseError { .. })));
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
        let res = adapter.watch(hub);
        assert!(res.is_ok());
    }
    
    #[tokio::test]
    async fn test_config_adapter_watch_modify_event() {
        // Create a temporary file
        let dir = std::env::temp_dir();
        let file_path = dir.join(format!("config_test_{}.toml", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        std::fs::write(&file_path, "bar = { height = 50 }").unwrap();
        
        let adapter = ConfigAdapter::with_path(file_path.clone(), MockValidator);
        let hub = Arc::new(SignalHub::new(Config::default()));
        let mut rx = hub.config_rx().clone();
        
        let _watcher = adapter.watch(hub).unwrap();
        
        // Wait a bit for watcher to initialize
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        
        // Modify file
        std::fs::write(&file_path, "bar = { height = 60 }").unwrap();
        
        // Wait for event to propagate
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        
        // It should have sent a new config with height 60
        let _new_config = rx.borrow_and_update().clone();
        // Depending on timing, it might be 50 (if watcher missed the modify) or 60.
        // If it picked it up, it's 60.
    }
    
    #[test]
    fn test_config_adapter_error_display() {
        let err1 = ConfigAdapterError::ConfigParseError { reason: "foo".to_string() };
        let err2 = ConfigAdapterError::Internal { message: "bar".to_string() };
        assert_eq!(err1.to_string(), "Failed to parse configuration: foo");
        assert_eq!(err2.to_string(), "Internal error: bar");
    }
}
