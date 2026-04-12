use log::info;
mod config;
mod core;
mod modules;
mod render;
mod utils;
#[cfg(test)]
#[macro_use]
pub mod test_utils;

fn get_config_path() -> std::path::PathBuf {
    std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default())
        .join(".config/cranky/config.toml")
}

fn load_config(path: &std::path::Path) -> Result<config::Config, Box<dyn std::error::Error>> {
    if path.exists() {
        Ok(config::Config::load_from_path(path)?)
    } else {
        info!("Config not found, using default placeholder config");
        Ok(config::Config::from_str(include_str!("../config.toml"))?)
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    pretty_env_logger::init();

    info!("Starting Cranky bar...");

    // Load config
    let config = load_config(&get_config_path())?;

    let wayland_manager = core::WaylandManager::new(config)?;
    wayland_manager.run().await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_config_path() {
        let old_home = std::env::var_os("HOME");
        unsafe {
            std::env::set_var("HOME", "/tmp/test-home");
        }

        let path = get_config_path();
        assert!(path.to_string_lossy().contains("/tmp/test-home"));

        if let Some(home) = old_home {
            unsafe {
                std::env::set_var("HOME", home);
            }
        }
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
}
