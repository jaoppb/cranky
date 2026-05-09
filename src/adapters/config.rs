use crate::config::{Config, ConfigError};
use crate::domain::signals::SignalHub;
use crate::domain::errors::DomainError;
use notify::{Event, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{info, error, info_span};

pub struct ConfigAdapter {
    config_path: PathBuf,
}

impl ConfigAdapter {
    pub fn new() -> Self {
        let config_path = std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default())
            .join(".config/cranky/config.toml");
        Self { config_path }
    }

    pub fn load_initial(&self) -> Result<Config, DomainError> {
        if self.config_path.exists() {
            Config::load_from_path(&self.config_path).map_err(|e| DomainError::ConfigParseError { 
                reason: e.to_string() 
            })
        } else {
            info!("Config not found at {:?}, using default placeholder config", self.config_path);
            Config::from_str(include_str!("../../config.toml")).map_err(|e| DomainError::ConfigParseError { 
                reason: e.to_string() 
            })
        }
    }

    pub fn watch(&self, hub: Arc<SignalHub>) -> Result<Box<dyn Watcher>, DomainError> {
        let config_tx = hub.config_tx();
        let path = self.config_path.clone();

        let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
            let span = info_span!("config_watch_event");
            let _enter = span.enter();
            match res {
                Ok(event) => {
                    if event.kind.is_modify() {
                        info!("Config file modified, reloading...");
                        if path.exists() {
                            match Config::load_from_path(&path) {
                                Ok(new_config) => {
                                    let _ = config_tx.send(new_config);
                                }
                                Err(e) => {
                                    error!("Failed to load updated config: {}", e);
                                }
                            }
                        }
                    }
                }
                Err(e) => error!("Config watcher error: {:?}", e),
            }
        }).map_err(|e| DomainError::Internal { message: format!("Failed to create watcher: {}", e) })?;

        if let Some(parent) = self.config_path.parent() {
            if parent.exists() {
                watcher.watch(parent, RecursiveMode::NonRecursive).map_err(|e| {
                    DomainError::Internal { message: format!("Failed to start watching config dir: {}", e) }
                })?;
            }
        }

        Ok(Box::new(watcher))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_adapter_load_initial_fallback() {
        let adapter = ConfigAdapter {
            config_path: PathBuf::from("/definitely/not/a/real/path/cranky.toml"),
        };

        // Should fallback to include_str!
        let config = adapter.load_initial().unwrap();
        assert_eq!(config.bar().height(), 40);
    }
}
