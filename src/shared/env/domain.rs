use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HomeDir(PathBuf);
impl HomeDir {
    #[must_use]
    pub const fn new(path: PathBuf) -> Self {
        Self(path)
    }

    #[must_use]
    pub const fn as_path(&self) -> &PathBuf {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XdgCacheHome(PathBuf);
impl XdgCacheHome {
    #[must_use]
    pub const fn new(path: PathBuf) -> Self {
        Self(path)
    }

    #[must_use]
    pub const fn as_path(&self) -> &PathBuf {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XdgRuntimeDir(PathBuf);
impl XdgRuntimeDir {
    #[must_use]
    pub const fn new(path: PathBuf) -> Self {
        Self(path)
    }

    #[must_use]
    pub const fn as_path(&self) -> &PathBuf {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RustLog(String);
impl RustLog {
    #[must_use]
    pub const fn new(log: String) -> Self {
        Self(log)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HyprlandInstanceSignature(String);
impl HyprlandInstanceSignature {
    #[must_use]
    pub const fn new(sig: String) -> Self {
        Self(sig)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug)]
pub struct AppEnvironment {
    home: HomeDir,
    xdg_cache_home: XdgCacheHome,
    xdg_runtime_dir: XdgRuntimeDir,
    rust_log: RustLog,
    hyprland_instance_signature: Option<HyprlandInstanceSignature>,
}

impl AppEnvironment {
    #[must_use]
    pub const fn new(
        home: HomeDir,
        xdg_cache_home: XdgCacheHome,
        xdg_runtime_dir: XdgRuntimeDir,
        rust_log: RustLog,
        hyprland_instance_signature: Option<HyprlandInstanceSignature>,
    ) -> Self {
        Self {
            home,
            xdg_cache_home,
            xdg_runtime_dir,
            rust_log,
            hyprland_instance_signature,
        }
    }

    #[must_use]
    pub const fn home(&self) -> &HomeDir {
        &self.home
    }

    #[must_use]
    pub const fn xdg_cache_home(&self) -> &XdgCacheHome {
        &self.xdg_cache_home
    }

    #[must_use]
    pub const fn xdg_runtime_dir(&self) -> &XdgRuntimeDir {
        &self.xdg_runtime_dir
    }

    #[must_use]
    pub const fn rust_log(&self) -> &RustLog {
        &self.rust_log
    }

    #[must_use]
    pub const fn hyprland_instance_signature(&self) -> Option<&HyprlandInstanceSignature> {
        self.hyprland_instance_signature.as_ref()
    }
}
