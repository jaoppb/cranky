use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum HyprError {
    #[error("Hyprland instance signature not found. Is Hyprland running?")]
    NoInstance,
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[cfg_attr(test, mockall::automock)]
pub trait HyprlandProvider: Send + Sync {
    /// Queries the monitors from Hyprland.
    ///
    /// # Errors
    ///
    /// Returns [`HyprError`] if the IPC communication fails.
    fn query_monitors(&self) -> Result<String, HyprError>;

    /// Queries the workspaces from Hyprland.
    ///
    /// # Errors
    ///
    /// Returns [`HyprError`] if the IPC communication fails.
    fn query_workspaces(&self) -> Result<String, HyprError>;

    /// Opens a stream to listen to Hyprland socket events.
    ///
    /// # Errors
    ///
    /// Returns [`HyprError`] if opening the event socket fails.
    fn listen_events(&self) -> Result<UnixStream, HyprError>;
}

pub struct RealHyprlandProvider {
    app_env: std::sync::Arc<crate::shared::env::domain::AppEnvironment>,
}

impl RealHyprlandProvider {
    #[must_use]
    pub const fn new(app_env: std::sync::Arc<crate::shared::env::domain::AppEnvironment>) -> Self {
        Self { app_env }
    }
}

impl HyprlandProvider for RealHyprlandProvider {
    fn query_monitors(&self) -> Result<String, HyprError> {
        query_socket("j/monitors", &self.app_env)
    }

    fn query_workspaces(&self) -> Result<String, HyprError> {
        query_socket("j/workspaces", &self.app_env)
    }

    fn listen_events(&self) -> Result<UnixStream, HyprError> {
        let signature = self
            .app_env
            .hyprland_instance_signature()
            .ok_or(HyprError::NoInstance)?;
        let xdg_runtime_dir = self.app_env.xdg_runtime_dir().as_path();

        let socket_path = PathBuf::from(xdg_runtime_dir)
            .join("hypr")
            .join(signature.as_str())
            .join(".socket2.sock");

        UnixStream::connect(socket_path).map_err(HyprError::Io)
    }
}

fn query_socket(
    command: &str,
    app_env: &crate::shared::env::domain::AppEnvironment,
) -> Result<String, HyprError> {
    let signature = app_env
        .hyprland_instance_signature()
        .ok_or(HyprError::NoInstance)?;
    let xdg_runtime_dir = app_env.xdg_runtime_dir().as_path();

    let socket_path = PathBuf::from(xdg_runtime_dir)
        .join("hypr")
        .join(signature.as_str())
        .join(".socket.sock");

    let mut stream = UnixStream::connect(socket_path)?;
    stream.write_all(command.as_bytes())?;

    let mut response = String::new();
    stream.read_to_string(&mut response)?;

    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hypr_error_display() {
        let err = HyprError::NoInstance;
        assert_eq!(
            format!("{err}"),
            "Hyprland instance signature not found. Is Hyprland running?"
        );

        let err = HyprError::Io(std::io::Error::other("test"));
        assert!(format!("{err}").contains("IO error: test"));
    }

    #[test]
    fn test_real_provider_paths() {
        let app_env = std::sync::Arc::new(crate::shared::env::domain::AppEnvironment::new(
            crate::shared::env::domain::HomeDir::new(std::path::PathBuf::from("/")),
            crate::shared::env::domain::XdgCacheHome::new(std::path::PathBuf::from("/")),
            crate::shared::env::domain::XdgRuntimeDir::new(std::path::PathBuf::from("/")),
            crate::shared::env::domain::RustLog::new(String::new()),
            None,
        ));
        let provider = RealHyprlandProvider::new(app_env);
        // These will likely fail in test env, but we want to exercise the wrapper logic
        let _ = provider.query_monitors();
        let _ = provider.query_workspaces();
    }
}
