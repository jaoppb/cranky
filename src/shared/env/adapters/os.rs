use crate::shared::env::domain::{
    AppEnvironment, HomeDir, HyprlandInstanceSignature, RustLog, XdgCacheHome, XdgRuntimeDir,
};
use crate::shared::env::ports::{EnvironmentError, EnvironmentPort};
use std::env;
use std::path::PathBuf;

pub struct OsEnvironmentAdapter;

impl OsEnvironmentAdapter {
    pub fn new() -> Self {
        Self
    }
}

impl Default for OsEnvironmentAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl EnvironmentPort for OsEnvironmentAdapter {
    fn read_environment(&self) -> Result<AppEnvironment, EnvironmentError> {
        let home =
            env::var("HOME").map_err(|_| EnvironmentError::MissingVariable("HOME".to_string()))?;
        let home_path = PathBuf::from(&home);

        let xdg_cache_home = env::var("XDG_CACHE_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| home_path.join(".cache"));

        let xdg_runtime_dir = env::var("XDG_RUNTIME_DIR")
            .map_err(|_| EnvironmentError::MissingVariable("XDG_RUNTIME_DIR".to_string()))?;

        let rust_log = env::var("RUST_LOG").unwrap_or_else(|_| "cranky=info".to_string());

        let hyprland_instance_signature = env::var("HYPRLAND_INSTANCE_SIGNATURE").ok();

        Ok(AppEnvironment::new(
            HomeDir::new(home_path),
            XdgCacheHome::new(xdg_cache_home),
            XdgRuntimeDir::new(PathBuf::from(xdg_runtime_dir)),
            RustLog::new(rust_log),
            hyprland_instance_signature.map(HyprlandInstanceSignature::new),
        ))
    }
}
