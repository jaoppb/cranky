#![deny(unsafe_code)]

use log::{error, info};
mod config;
mod core;
mod modules;
mod render;
mod utils;
#[cfg(test)]
#[macro_use]
pub mod test_utils;

use config::{Config, ConfigError, ReloadError};
use notify::{Event, RecursiveMode, Watcher};
use tokio::sync::mpsc;

fn get_config_path() -> std::path::PathBuf {
    std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default())
        .join(".config/cranky/config.toml")
}

fn load_config(path: &std::path::Path) -> Result<Config, ConfigError> {
    if path.exists() {
        Config::load_from_path(path)
    } else {
        info!("Config not found, using default placeholder config");
        Config::from_str(include_str!("../config.toml"))
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    pretty_env_logger::init();

    info!("Starting Cranky bar...");

    let config_path = get_config_path();
    // Load initial config
    let config = load_config(&config_path)?;

    let (tx, rx) = mpsc::channel(1);

    let config_path_clone = config_path.clone();
    let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| match res {
        Ok(event) => {
            if event.kind.is_modify() {
                info!("Config file modified, reloading...");
                match load_config(&config_path_clone) {
                    Ok(new_config) => {
                        let _ = tx.blocking_send(Ok(new_config));
                    }
                    Err(e) => {
                        error!("Failed to load updated config: {}", e);
                        let _ = tx.blocking_send(Err(ReloadError::from(e)));
                    }
                }
            }
        }
        Err(e) => error!("watch error: {:?}", e),
    })?;

    if let Some(parent) = config_path.parent()
        && parent.exists() {
            watcher.watch(parent, RecursiveMode::NonRecursive)?;
        }

    let wayland_manager = core::WaylandManager::new(config)?;
    wayland_manager.run(rx).await?;

    Ok(())
}

#[cfg(test)]
#[allow(unsafe_code)]
mod tests {
    use super::*;

    fn with_env<F: FnOnce()>(key: &str, value: Option<&str>, f: F) {
        let old_value = std::env::var_os(key);
        if let Some(val) = value {
            unsafe {
                std::env::set_var(key, val);
            }
        } else {
            unsafe {
                std::env::remove_var(key);
            }
        }
        f();
        if let Some(val) = old_value {
            unsafe {
                std::env::set_var(key, val);
            }
        } else {
            unsafe {
                std::env::remove_var(key);
            }
        }
    }

    #[test]
    fn test_get_config_path() {
        with_env("HOME", Some("/tmp/test-home"), || {
            let path = get_config_path();
            assert!(path.to_string_lossy().contains("/tmp/test-home"));
        });
    }

    #[test]
    fn test_load_config_default() {
        // Path that definitely doesn't exist
        let path = std::path::PathBuf::from("/non-existent-path-12345");
        let config = load_config(&path).unwrap();
        assert_eq!(config.bar().height(), 40);
    }

    #[test]
    fn test_load_config_exists() {
        let temp_dir = std::env::temp_dir();
        let config_path = temp_dir.join("cranky-test-config.toml");
        let config_content = r##"
            [bar]
            background = "#123456"
            height = 42
            [modules]
        "##;
        std::fs::write(&config_path, config_content).unwrap();

        let config = load_config(&config_path).unwrap();
        assert_eq!(
            config.bar().background(),
            &crate::utils::ParsedColor::try_from("#123456").unwrap()
        );
        assert_eq!(config.bar().height(), 42);

        let _ = std::fs::remove_file(config_path);
    }

    #[test]
    fn test_get_config_path_without_home() {
        with_env("HOME", None, || {
            let path = get_config_path();
            assert!(
                path.to_string_lossy()
                    .ends_with(".config/cranky/config.toml")
            );
        });
    }

    #[test]
    fn test_load_config_exists_invalid() {
        let temp_dir = std::env::temp_dir();
        let config_path = temp_dir.join("cranky-test-invalid-config.toml");
        std::fs::write(&config_path, "[bar]\nheight = \"bad\"\n").unwrap();
        let config = load_config(&config_path);
        let _ = std::fs::remove_file(config_path);
        assert!(config.is_err());
    }

    #[test]
    fn test_load_config_non_existent() {
        let path = std::path::PathBuf::from("/tmp/should-not-exist-cranky-config.toml");
        if path.exists() {
            let _ = std::fs::remove_file(&path);
        }
        let config = load_config(&path);
        assert!(config.is_ok());
    }

    #[test]
    fn test_error_displays() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "test");
        let cfg_err = ConfigError::Io(io_err);
        assert!(format!("{}", cfg_err).contains("IO error"));
        assert!(format!("{:?}", cfg_err).contains("Io"));
        
        let reload_err = ReloadError::Config(cfg_err);
        assert!(format!("{}", reload_err).contains("config"));
        assert!(format!("{:?}", reload_err).contains("Config"));
    }
}
