use std::env;
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
    fn query_monitors(&self) -> Result<String, HyprError>;
    fn query_workspaces(&self) -> Result<String, HyprError>;
    fn listen_events(&self) -> Result<UnixStream, HyprError>;
}

pub struct RealHyprlandProvider;

impl HyprlandProvider for RealHyprlandProvider {
    fn query_monitors(&self) -> Result<String, HyprError> {
        query_socket("j/monitors")
    }

    fn query_workspaces(&self) -> Result<String, HyprError> {
        query_socket("j/workspaces")
    }

    fn listen_events(&self) -> Result<UnixStream, HyprError> {
        let signature =
            env::var("HYPRLAND_INSTANCE_SIGNATURE").map_err(|_| HyprError::NoInstance)?;
        let xdg_runtime_dir = env::var("XDG_RUNTIME_DIR").map_err(|_| HyprError::NoInstance)?;

        let socket_path = PathBuf::from(xdg_runtime_dir)
            .join("hypr")
            .join(signature)
            .join(".socket2.sock");

        UnixStream::connect(socket_path).map_err(HyprError::Io)
    }
}

fn query_socket(command: &str) -> Result<String, HyprError> {
    let signature = env::var("HYPRLAND_INSTANCE_SIGNATURE").map_err(|_| HyprError::NoInstance)?;
    let xdg_runtime_dir = env::var("XDG_RUNTIME_DIR").map_err(|_| HyprError::NoInstance)?;

    let socket_path = PathBuf::from(xdg_runtime_dir)
        .join("hypr")
        .join(signature)
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
            format!("{}", err),
            "Hyprland instance signature not found. Is Hyprland running?"
        );

        let err = HyprError::Io(std::io::Error::other("test"));
        assert!(format!("{}", err).contains("IO error: test"));
    }

    #[test]
    fn test_real_provider_paths() {
        let provider = RealHyprlandProvider;
        // These will likely fail in test env, but we want to exercise the wrapper logic
        let _ = provider.query_monitors();
        let _ = provider.query_workspaces();
    }
}
