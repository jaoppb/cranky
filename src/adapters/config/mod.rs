pub mod dto;

use crate::domain::config::Config;
use crate::domain::signals::SignalHub;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigAdapterError {
    #[error("Failed to parse configuration: {reason}")]
    ConfigParseError { reason: String },
    #[error("Internal error: {message}")]
    Internal { message: String },
}

use crate::adapters::config::dto::ConfigDto;
use crate::ports::font::FontValidatorPort;
use notify::{Event, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{error, info, info_span};

pub struct ConfigAdapter<V: FontValidatorPort + Send + Sync + 'static> {
    config_path: PathBuf,
    validator: Arc<V>,
}

impl<V: FontValidatorPort + Send + Sync + 'static> ConfigAdapter<V> {
    pub fn new(validator: V) -> Self {
        let config_path = std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default())
            .join(".config/cranky/config.toml");
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
            self.load_from_str(include_str!("../../../config.toml"))
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
        assert_eq!(config.bar().height(), 40); // config.toml height is 40
    }
}
