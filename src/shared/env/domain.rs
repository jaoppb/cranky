//! The environment variables Cranky depends on, each wrapped in its own type.
use std::path::PathBuf;

/// The user's `$HOME`, root of the `~/.config/cranky` and `~/.local/share/cranky` paths.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HomeDir(PathBuf);
impl HomeDir {
    /// Wraps the path as read; it is not checked to exist.
    #[must_use]
    pub const fn new(path: PathBuf) -> Self {
        Self(path)
    }

    #[must_use]
    pub(crate) const fn as_path(&self) -> &PathBuf {
        &self.0
    }
}

/// `$XDG_CACHE_HOME`, or `~/.cache` when unset; the log files are written under it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XdgCacheHome(PathBuf);
impl XdgCacheHome {
    /// Wraps the path as resolved; it is not checked to exist.
    #[must_use]
    pub const fn new(path: PathBuf) -> Self {
        Self(path)
    }

    #[must_use]
    pub(crate) const fn as_path(&self) -> &PathBuf {
        &self.0
    }
}

/// `$XDG_RUNTIME_DIR`, where the shared-memory buffer files are created and the Hyprland IPC
/// sockets live; required.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XdgRuntimeDir(PathBuf);
impl XdgRuntimeDir {
    /// Wraps the path as read; it is not checked to exist.
    #[must_use]
    pub const fn new(path: PathBuf) -> Self {
        Self(path)
    }

    #[must_use]
    pub(crate) const fn as_path(&self) -> &PathBuf {
        &self.0
    }
}

/// Tracing filter directive from `$RUST_LOG`, `cranky=info` when unset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RustLog(String);
impl RustLog {
    /// Wraps the directive as read; tracing setup parses it.
    #[must_use]
    pub const fn new(log: String) -> Self {
        Self(log)
    }

    #[must_use]
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

/// `$HYPRLAND_INSTANCE_SIGNATURE`, naming the directory of the running Hyprland's IPC sockets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HyprlandInstanceSignature(String);
impl HyprlandInstanceSignature {
    #[must_use]
    pub(crate) const fn new(sig: String) -> Self {
        Self(sig)
    }

    #[must_use]
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

/// Everything the app reads from the process environment, captured once at startup.
#[derive(Debug)]
pub struct AppEnvironment {
    home: HomeDir,
    xdg_cache_home: XdgCacheHome,
    xdg_runtime_dir: XdgRuntimeDir,
    rust_log: RustLog,
    hyprland_instance_signature: Option<HyprlandInstanceSignature>,
}

impl AppEnvironment {
    /// Bundles already-read values; the Hyprland signature is `None` outside a Hyprland session.
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
    pub(crate) const fn home(&self) -> &HomeDir {
        &self.home
    }

    #[must_use]
    pub(crate) const fn xdg_cache_home(&self) -> &XdgCacheHome {
        &self.xdg_cache_home
    }

    #[must_use]
    pub(crate) const fn xdg_runtime_dir(&self) -> &XdgRuntimeDir {
        &self.xdg_runtime_dir
    }

    #[must_use]
    pub(crate) const fn rust_log(&self) -> &RustLog {
        &self.rust_log
    }

    #[must_use]
    pub(crate) const fn hyprland_instance_signature(&self) -> Option<&HyprlandInstanceSignature> {
        self.hyprland_instance_signature.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

    #[test]
    fn test_app_environment_keeps_each_value_in_its_place() {
        let env = AppEnvironment::new(
            HomeDir::new(PathBuf::from("/home/someone")),
            XdgCacheHome::new(PathBuf::from("/var/cache/someone")),
            XdgRuntimeDir::new(PathBuf::from("/run/user/1000")),
            RustLog::new("cranky=debug".to_string()),
            Some(HyprlandInstanceSignature::new("abc123".to_string())),
        );
        assert_eq!(env.home().as_path(), Path::new("/home/someone"));
        assert_eq!(
            env.xdg_cache_home().as_path(),
            Path::new("/var/cache/someone")
        );
        assert_eq!(env.xdg_runtime_dir().as_path(), Path::new("/run/user/1000"));
        assert_eq!(env.rust_log().as_str(), "cranky=debug");
        assert_eq!(
            env.hyprland_instance_signature()
                .map(HyprlandInstanceSignature::as_str),
            Some("abc123")
        );
    }
}
