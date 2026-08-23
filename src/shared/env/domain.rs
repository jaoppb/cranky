use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HomeDir(PathBuf);
impl HomeDir {
    pub fn new(path: PathBuf) -> Self {
        Self(path)
    }
    pub fn as_path(&self) -> &PathBuf {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XdgCacheHome(PathBuf);
impl XdgCacheHome {
    pub fn new(path: PathBuf) -> Self {
        Self(path)
    }
    pub fn as_path(&self) -> &PathBuf {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XdgRuntimeDir(PathBuf);
impl XdgRuntimeDir {
    pub fn new(path: PathBuf) -> Self {
        Self(path)
    }
    pub fn as_path(&self) -> &PathBuf {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RustLog(String);
impl RustLog {
    pub fn new(log: String) -> Self {
        Self(log)
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HyprlandInstanceSignature(String);
impl HyprlandInstanceSignature {
    pub fn new(sig: String) -> Self {
        Self(sig)
    }
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
    pub fn new(
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

    pub fn home(&self) -> &HomeDir {
        &self.home
    }
    pub fn xdg_cache_home(&self) -> &XdgCacheHome {
        &self.xdg_cache_home
    }
    pub fn xdg_runtime_dir(&self) -> &XdgRuntimeDir {
        &self.xdg_runtime_dir
    }
    pub fn rust_log(&self) -> &RustLog {
        &self.rust_log
    }
    pub fn hyprland_instance_signature(&self) -> Option<&HyprlandInstanceSignature> {
        self.hyprland_instance_signature.as_ref()
    }
}
